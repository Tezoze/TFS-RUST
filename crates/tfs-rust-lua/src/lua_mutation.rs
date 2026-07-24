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
    /// `Creature::addCondition`. LUA-4 / PC-3a Phase 3: full builder fields via
    /// [`ConditionApplySpec`].
    PlayerAddCondition {
        creature_id: u64,
        spec: ConditionApplySpec,
    },
    /// `player:removeCondition(type, id, subId)` — `luascript.cpp:2118`
    /// `Creature::removeCondition`. LUA-4; immediate apply.
    PlayerRemoveCondition {
        creature_id: u64,
        ctype: i32,
        cond_id: i32,
        sub_id: u32,
    },
    /// `player:setInFight(bool)` — TFS `Player::onAttackedCreature` / PZ-lock
    /// fight flag. PC-3a Phase 3: `poison_storm.lua` sets in-fight when a
    /// player is hit by the AoE poison.
    PlayerSetInFight {
        creature_id: u64,
        in_fight: bool,
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
    /// `item:decay()` — TFS `luaItemDecay` → `Game::startDecay`.
    ItemDecay {
        item_id: u64,
    },
    /// `player:addMana(manaChange)` — `luascript.cpp` `luaPlayerAddMana`.
    /// PC-3a Phase 5: `conjureItem` dual-hand second-conjure mana deduction.
    /// Clamps to `[0, max_mana]`; no combat animation path.
    PlayerAddMana {
        creature_id: u64,
        mana_change: i32,
    },
    /// `player:addManaSpent(amount)` — `luascript.cpp` `luaPlayerAddManaSpent`.
    /// PC-3a Phase 5: advances magic level via `Player::magic_increase`.
    PlayerAddManaSpent {
        creature_id: u64,
        amount: u64,
    },
    /// `item:transform(itemId[, count/subType])` — `luascript.cpp`
    /// `luaItemTransform` → `Game::transformItem`. PC-3a Phase 5: in-place
    /// type/subtype change + cylinder notify (inventory/container).
    ItemTransform {
        item_id: u64,
        new_type: u16,
        sub_type: i32,
    },
    /// `combat:execute(creature, variant)` — PC-3a. The Lua side resolves area
    /// offsets + formula min/max, then the core applier iterates tiles, checks
    /// `throw_possible`, and applies damage via `combat_execute_with_stimulus`.
    /// C++ reference: `luascript.cpp:13198` `luaCombatExecute` → `Combat::doCombat`.
    CombatExecute {
        request: CombatExecuteRequest,
    },
    /// `doChallengeCreature(creature, target)` — PC-3a Phase 6.
    /// C++ `luaDoChallengeCreature` → `Monster::challengeCreature` (`monster.cpp:2070`).
    ChallengeCreature {
        challenger_id: u64,
        target_id: u64,
    },
    /// `Game.createMonster(name, pos[, extended[, force]])` — PC-3a Gap 5.
    /// C++ `luascript.cpp` `luaGameCreateMonster`. Result creature id via
    /// [`set_mutation_item_result`] (None → Lua nil).
    CreateMonster {
        name: String,
        x: u16,
        y: u16,
        z: u8,
        extended: bool,
        force: bool,
    },
    /// `creature:addSummon(monster)` — set master + clear target/follow.
    AddSummon {
        master_id: u64,
        summon_id: u64,
    },
    /// `creature:move(tile, flags)` — `Game::internalMoveCreature` to tile.
    CreatureMoveToTile {
        creature_id: u64,
        x: u16,
        y: u16,
        z: u8,
        flags: u32,
    },
    /// `creature:teleportTo(pos[, pushMovement])` — `Game::internalTeleport`.
    CreatureTeleport {
        creature_id: u64,
        x: u16,
        y: u16,
        z: u8,
        push_movement: bool,
    },
    /// `creature:sendTextMessage(type, text)` — `Player::sendTextMessage`.
    PlayerSendTextMessage {
        creature_id: u64,
        msg_class: u8,
        text: String,
    },
    /// NPC-7: `npc:say(text)` — schedule NPC speech via ToDo / immediate say path.
    NpcSay {
        npc_id: u64,
        text: String,
    },
    /// NPC-7: `npc:setFocus(player|nil)`.
    NpcSetFocus {
        npc_id: u64,
        player_id: Option<u64>,
    },
    /// NPC-7: deposit gold into `PlayerEconomy.balance` (removes coins first).
    PlayerBankDeposit {
        creature_id: u64,
        amount: u64,
    },
    /// NPC-7: withdraw from bank into inventory money.
    PlayerBankWithdraw {
        creature_id: u64,
        amount: u64,
    },
}

/// Snapshot of a Lua `ConditionBuilder` for the mutation / combat-execute seam.
/// PC-3a Phases 2–3: core maps this to `ActiveCondition` via
/// `active_condition_from_apply_spec`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConditionApplySpec {
    /// 772 bit-flag `ConditionType_t` (e.g. `CONDITION_LIGHT = 1<<8`).
    pub ctype: i32,
    pub cond_id: i32,
    pub sub_id: u32,
    /// Duration in ms (`CONDITION_PARAM_TICKS` / `setTicks`).
    pub ticks: i32,
    pub speed: i32,
    pub light_level: i32,
    pub light_color: i32,
    /// 772 DoT cycle strength (`CONDITION_PARAM_CYCLE`).
    pub cycle: i32,
    pub count: i32,
    pub max_count: i32,
    pub look_type: i32,
    pub health_gain: i32,
    pub health_ticks: i32,
    pub mana_gain: i32,
    pub mana_ticks: i32,
}

/// Parameters for `Combat:execute()` — PC-3a. Built by the Lua `Combat:execute()`
/// method from `CombatDef` + variant resolution, passed to the core applier.
///
/// C++ reference: `Combat::doCombat` / `doAreaCombat` — `combat.cpp:683,737,929`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatExecuteRequest {
    /// Caster creature ID (slotmap key bits as u64).
    pub caster_id: u64,
    /// Spell center position (target tile or caster position).
    pub center_x: u16,
    pub center_y: u16,
    pub center_z: u8,
    /// Caster position — used for LoS checks on directional spells.
    /// 772 `AngleShapeSpell` uses `ThrowPossible(ActorX, ActorY, ActorZ, ...)`
    /// (from caster), while `ExecuteCircleSpell` uses `ThrowPossible(DestX, ...)`
    /// (from center). When center == caster (non-directional), these are equal.
    pub caster_x: u16,
    pub caster_y: u16,
    pub caster_z: u8,
    /// Combat damage type bit-flag (COMBAT_PHYSICALDAMAGE=1, etc.).
    pub combat_type: i32,
    /// Magic effect byte (CONST_ME_*).
    pub effect: i32,
    /// Whether the combat is aggressive (PZ lock / PVP gate).
    pub aggressive: bool,
    /// Whether armor is applied.
    pub block_armor: bool,
    /// Whether shield defense is applied.
    pub block_shield: bool,
    /// Resolved area offsets relative to center (from matrix or ring).
    pub area_offsets: Vec<(i32, i32)>,
    /// Pre-computed damage min/max (from formula + callback resolution).
    pub damage_min: i32,
    pub damage_max: i32,
    /// Conditions from `combat:addCondition` — applied per target after damage.
    /// C++ `CombatParams::conditionList` — `combat.h:44`.
    pub conditions: Vec<ConditionApplySpec>,
    /// `COMBAT_PARAM_DISPEL` — 772 bit-flag condition type to remove per target.
    /// `None` / `0` means no dispel. C++ `CombatParams::dispelType` — `combat.h:52`.
    pub dispel_type: Option<i32>,
    /// `COMBAT_PARAM_CREATEITEM` — item type id to place on affected tiles.
    /// `0` = none. C++ `CombatParams::itemId` — `combat.h` / `combatTileEffects`.
    pub create_item: i32,
    /// `COMBAT_PARAM_NODAMAGE` — skip damage roll (FX / CREATEITEM / conditions still run).
    /// C++ `CombatParams` path when damage is suppressed — `combat.h`.
    pub no_damage: bool,
    /// `COMBAT_PARAM_DISTANCEEFFECT` — shoot type (`CONST_ANI_*`); `0` = none.
    /// C++ `CombatParams::distanceEffect` / `postCombatEffects` — `combat.cpp:643`.
    pub distance_effect: i32,
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
    apply_mutation(LuaMutation::ItemSetActionId { item_id, action_id })
}

pub fn call_lua_set_unique_id(item_id: u64, unique_id: u16) -> Result<(), String> {
    apply_mutation(LuaMutation::ItemSetUniqueId { item_id, unique_id })
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
    apply_mutation(LuaMutation::PlayerSendCancelMessage { creature_id, text })
}

/// `player:addCondition(condition)` — LUA-4 / PC-3a Phase 3. Immediate-apply
/// condition add with full [`ConditionApplySpec`] fields.
pub fn call_lua_add_condition(
    creature_id: u64,
    spec: ConditionApplySpec,
) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerAddCondition { creature_id, spec })
}

/// `player:setInFight(bool)` — PC-3a Phase 3 (`poison_storm.lua`).
pub fn call_lua_set_in_fight(creature_id: u64, in_fight: bool) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerSetInFight {
        creature_id,
        in_fight,
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

/// `item:decay()` — schedules via core `GameWorld::start_decay`.
pub fn call_lua_item_decay(item_id: u64) -> Result<(), String> {
    apply_mutation(LuaMutation::ItemDecay { item_id })
}

/// `player:addMana(manaChange)` — PC-3a Phase 5. Clamps mana to `[0, max_mana]`.
/// C++ `luascript.cpp` `luaPlayerAddMana` → `Player::changeMana`.
pub fn call_lua_add_mana(creature_id: u64, mana_change: i32) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerAddMana {
        creature_id,
        mana_change,
    })
}

/// `player:addManaSpent(amount)` — PC-3a Phase 5. Advances magic level.
/// C++ `luascript.cpp` `luaPlayerAddManaSpent` → `Player::addManaSpent`.
pub fn call_lua_add_mana_spent(creature_id: u64, amount: u64) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerAddManaSpent {
        creature_id,
        amount,
    })
}

/// `item:transform(itemId[, count/subType])` — PC-3a Phase 5.
/// C++ `luascript.cpp` `luaItemTransform` → `Game::transformItem`.
pub fn call_lua_item_transform(
    item_id: u64,
    new_type: u16,
    sub_type: i32,
) -> Result<bool, String> {
    apply_mutation(LuaMutation::ItemTransform {
        item_id,
        new_type,
        sub_type,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// `combat:execute(creature, variant)` — PC-3a. Synchronous AoE combat
/// execution. C++ reference: `luascript.cpp:13198` `luaCombatExecute` →
/// `Combat::doCombat` (`combat.cpp:683,737`). The core applier iterates
/// `area_offsets`, checks `throw_possible` per tile, and applies damage.
pub fn call_combat_execute(request: CombatExecuteRequest) -> Result<(), String> {
    apply_mutation(LuaMutation::CombatExecute { request })
}

/// `doChallengeCreature(creature, target)` — PC-3a Phase 6.
/// C++ `luascript.cpp` `luaDoChallengeCreature` → `Monster::challengeCreature`.
pub fn call_do_challenge_creature(challenger_id: u64, target_id: u64) -> Result<bool, String> {
    apply_mutation(LuaMutation::ChallengeCreature {
        challenger_id,
        target_id,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// `Game.createMonster` — PC-3a Gap 5. Returns created creature id bits, or None.
pub fn call_create_monster(
    name: String,
    x: u16,
    y: u16,
    z: u8,
    extended: bool,
    force: bool,
) -> Result<Option<u64>, String> {
    apply_mutation(LuaMutation::CreateMonster {
        name,
        x,
        y,
        z,
        extended,
        force,
    })?;
    Ok(take_mutation_item_result())
}

pub fn call_add_summon(master_id: u64, summon_id: u64) -> Result<bool, String> {
    apply_mutation(LuaMutation::AddSummon {
        master_id,
        summon_id,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

pub fn call_creature_move_to_tile(
    creature_id: u64,
    x: u16,
    y: u16,
    z: u8,
    flags: u32,
) -> Result<bool, String> {
    apply_mutation(LuaMutation::CreatureMoveToTile {
        creature_id,
        x,
        y,
        z,
        flags,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

pub fn call_creature_teleport(
    creature_id: u64,
    x: u16,
    y: u16,
    z: u8,
    push_movement: bool,
) -> Result<bool, String> {
    apply_mutation(LuaMutation::CreatureTeleport {
        creature_id,
        x,
        y,
        z,
        push_movement,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

pub fn call_send_text_message(
    creature_id: u64,
    msg_class: u8,
    text: String,
) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerSendTextMessage {
        creature_id,
        msg_class,
        text,
    })
}

pub fn call_lua_npc_say(npc_id: u64, text: &str) -> Result<(), String> {
    apply_mutation(LuaMutation::NpcSay {
        npc_id,
        text: text.to_string(),
    })
}

pub fn call_lua_npc_set_focus(npc_id: u64, player_id: Option<u64>) -> Result<(), String> {
    apply_mutation(LuaMutation::NpcSetFocus { npc_id, player_id })
}

pub fn call_lua_bank_deposit(creature_id: u64, amount: u64) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerBankDeposit {
        creature_id,
        amount,
    })
}

pub fn call_lua_bank_withdraw(creature_id: u64, amount: u64) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerBankWithdraw {
        creature_id,
        amount,
    })
}
