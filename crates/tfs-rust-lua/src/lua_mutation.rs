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
    PlayerGetDepotLocker {
        creature_id: u64,
        depot_id: u32,
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
    /// `item:setAttribute("keynumber", n)` — Remere custom attrs.
    ItemSetCustomAttribute {
        item_id: u64,
        key: String,
        /// Integer payload (doors/keys); extend later for string/float if needed.
        value: i64,
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
    /// `creature:addHealth(healthChange)` — TFS `luaCreatureAddHealth`.
    /// E4: HP clamp like `addMana` (not `combatChangeHealth`). 772 `Heal`
    /// in `DrinkPotion` (`magic.cc:2086`, `crskill.cc:58` `TSkill::Change`).
    PlayerAddHealth {
        creature_id: u64,
        health_change: i32,
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
    /// `Game.startRaid(name)` — TFS `luaGameStartRaid`.
    StartRaid {
        name: String,
    },
    /// `Game.createNpc(name, pos[, force])` — TFS `luaGameCreateNpc`.
    CreateNpc {
        name: String,
        x: u16,
        y: u16,
        z: u8,
        force: bool,
    },
    /// `npc:setMasterPos(pos[, radius])` — TFS `luaNpcSetMasterPos`.
    NpcSetMasterPos {
        npc_id: u64,
        x: u16,
        y: u16,
        z: u8,
        radius: Option<u16>,
    },
    /// `Game.setGameState(state)` — TFS `luaGameSetGameState`.
    SetGameState {
        state: i32,
    },
    /// `Game.reload(type)` — TFS `luaGameReload` (slim).
    GameReload {
        reload_type: i32,
    },
    /// `player:setSex(sex)` — TFS `luaPlayerSetSex`.
    PlayerSetSex {
        creature_id: u64,
        sex: u8,
    },
    /// `party:setSharedExperience(enabled)` — TFS `luaPartySetSharedExperience`.
    PartySetSharedExperience {
        party_id: u32,
        enabled: bool,
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
    /// `player:removeMoney(amount)` — inventory coins only (`Player::removeMoney`).
    PlayerRemoveMoney {
        creature_id: u64,
        amount: u64,
    },
    /// `player:setBankBalance(balance)` — TFS `luaPlayerSetBankBalance`.
    PlayerSetBankBalance {
        creature_id: u64,
        balance: u64,
    },
    /// `player:setPremiumEndsAt(timestamp)` — `Player::setPremiumTime` + DB.
    PlayerSetPremiumEndsAt {
        creature_id: u64,
        ends_at: u32,
    },
    /// `player:setStorageValue(key, value)` — `Player::addStorageValue` (`player.cpp`).
    /// `value == -1` erases the key.
    PlayerSetStorageValue {
        creature_id: u64,
        key: u32,
        value: i32,
    },
    /// `player:setTown(town)` — `Player::setTown` (`player.h`). In-memory
    /// `town_id` only; no teleport. TFS `luaPlayerSetTown`.
    PlayerSetTown {
        creature_id: u64,
        town_id: u32,
    },
    /// `setWorldLight(level, color)` — TFS `LuaScriptInterface::luaSetWorldLight`.
    /// C++ ref: `gameserver/src/luascript.cpp:3132-3145`.
    SetWorldLight {
        level: u8,
        color: u8,
    },
    /// 772 `ClearField` before door close — shove stack mates off the door tile.
    /// `exclude_item_id` is the door (not moved); optional creature exclude for SeparationEvent.
    ClearField {
        exclude_item_id: u64,
        exclude_creature_id: Option<u64>,
    },
    /// `player:addSkillTries(skill, tries)` — `luascript.cpp` `luaPlayerAddSkillTries`
    /// → `Player::addSkillAdvance`. Does **not** apply `rateSkill` (the data-pack
    /// wrapper sets `APPLY_SKILL_MULTIPLIER = false`).
    PlayerAddSkillTries {
        creature_id: u64,
        skill: i32,
        tries: u64,
    },
    /// `tile:addItem(itemId[, count[, flags]])` — `luascript.cpp` `luaTileAddItem`.
    TileAddItem {
        x: u16,
        y: u16,
        z: u8,
        item_type: u16,
        count: u16,
        flags: u32,
    },
    /// `Game.createItem(itemId[, count[, position]])` — `luascript.cpp` `luaGameCreateItem`.
    /// `position = None` leaves the item detached (virtual cylinder).
    GameCreateItem {
        item_type: u16,
        count: u16,
        position: Option<(u16, u16, u8)>,
    },
    /// `doTargetCombat` / `doTargetCombatHealth` — `luascript.cpp` `luaDoTargetCombat`.
    /// Single-target only; `attacker_id = None` is environment damage (`cid == 0`).
    TargetCombatHealth {
        attacker_id: Option<u64>,
        target_id: u64,
        combat_type: i32,
        damage_min: i32,
        damage_max: i32,
        effect: i32,
    },
    /// `creature:say(text[, type])` — E5. Viewport broadcast only; does **not**
    /// parse spells (`player_say`). 772 `Talk` (`TALK_SAY=1`); TFS `luaCreatureSay`.
    PlayerSay {
        creature_id: u64,
        text: String,
        speak_type: u8,
    },
    /// `player:showTextDialog(itemId, text)` — E6. `0x96` text window.
    /// TFS `luaPlayerShowTextDialog`; 772 `SendEditText` (`sending.cc:1088`).
    PlayerShowTextDialog {
        creature_id: u64,
        item_type: u16,
        text: String,
    },
    /// `player:addItemEx` / `container:addItemEx` / `tile:addItemEx`.
    /// TFS `luaPlayerAddItemEx` / `luaContainerAddItemEx` / `luaTileAddItemEx`.
    /// Detached item only (`parent == None` = VirtualCylinder). Result is `RETURNVALUE_*`.
    AddItemEx {
        item_id: u64,
        dest: LuaMoveDestination,
        can_drop_on_map: bool,
        index: i32,
        flags: u32,
    },
    /// `Game.createTile(position[, isDynamic])` / `Game.createTile(x, y, z[, isDynamic])`.
    /// TFS `luaGameCreateTile` — get-or-create tile. `is_dynamic` is accepted (TFS
    /// DynamicTile vs StaticTile) but unused: we only have `Tile::Normal` / `House`.
    GameCreateTile {
        x: u16,
        y: u16,
        z: u8,
        is_dynamic: bool,
    },
    /// `player:registerEvent(name)` / `unregisterEvent(name)` — TFS `luaPlayerRegisterEvent`.
    PlayerRegisterCreatureEvent {
        creature_id: u64,
        name: String,
        register: bool,
    },
    /// `player:sendOutfitWindow()` — TFS `luaPlayerSendOutfitWindow`.
    PlayerSendOutfitWindow {
        creature_id: u64,
    },
    /// `player:setOutfit(table)` — TFS `luaPlayerSetOutfit`.
    PlayerSetOutfit {
        creature_id: u64,
        look_type: i32,
        look_head: i32,
        look_body: i32,
        look_legs: i32,
        look_feet: i32,
        look_addons: i32,
    },
    /// `player:setVocation(id)` — TFS `luaPlayerSetVocation`.
    PlayerSetVocation {
        creature_id: u64,
        vocation_id: i32,
    },
    /// `creature:setDirection(dir)` — TFS `luaCreatureSetDirection`.
    PlayerSetDirection {
        creature_id: u64,
        direction: u8,
    },
    /// `saveServer()` / `Game.saveServer()` — TFS `luaSaveServer` → `Game::saveGameState`.
    SaveServer,
    /// `player:setGhostMode(enabled)` — TFS `luaPlayerSetGhostMode`.
    PlayerSetGhostMode {
        creature_id: u64,
        enabled: bool,
    },
    /// `creature:remove()` — TFS `luaCreatureRemove` (kick / despawn; not `item:remove`).
    CreatureRemove {
        creature_id: u64,
    },
    /// `house:setOwnerGuid(guid)` — TFS `luaHouseSetOwnerGuid`.
    HouseSetOwner {
        house_id: u32,
        guid: u32,
    },
    /// `house:setAccessList(listId, text)` — TFS `luaHouseSetAccessList`.
    HouseSetAccessList {
        house_id: u32,
        list_id: u32,
        text: String,
    },
    /// `house:kickPlayer(player, target)` — TFS `luaHouseKickPlayer`.
    HouseKickPlayer {
        house_id: u32,
        kicker_id: u64,
        target_id: u64,
    },
    /// `house:save()` — TFS `luaHouseSave` (marks house info dirty for persist).
    HouseSave {
        house_id: u32,
    },
    /// `player:setEditHouse(house, listId)` — TFS `luaPlayerSetEditHouse`.
    PlayerSetEditHouse {
        creature_id: u64,
        house_id: u32,
        list_id: u32,
    },
    /// `player:sendHouseWindow(house, listId)` — TFS `luaPlayerSendHouseWindow`.
    PlayerSendHouseWindow {
        creature_id: u64,
        house_id: u32,
        list_id: u32,
    },
    /// Native pack helpers (`onUseQuest` / `destroyItem` / `onUse*` / `checkScarabTile`).
    ToolUse {
        request: ToolUseRequest,
    },
    /// Native `Player:conjureItem`.
    ConjureItem {
        request: ConjureRequest,
    },
}

/// Quest chest reward row — `functions.lua` `chest.item` / `chest.content[]`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuestRewardSpec {
    pub id: u16,
    pub count: Option<u16>,
    pub subtype: Option<u16>,
    pub charges: Option<u16>,
    pub text: Option<String>,
    pub keynumber: Option<i64>,
}

/// Parsed `onUseQuest` chest table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuestChestSpec {
    pub storage_value: u32,
    pub item: QuestRewardSpec,
    pub content: Vec<QuestRewardSpec>,
}

/// One native tool-use helper. Inner op keeps a single `LuaMutation` variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolUseKind {
    DestroyItem,
    Machete,
    Pick,
    Knife,
    Rope,
    Shovel,
    Scythe,
    Quest,
    CheckScarab,
}

/// Arguments for [`LuaMutation::ToolUse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolUseRequest {
    pub kind: ToolUseKind,
    pub player: u64,
    /// Used item (tool / chest). Unused for `checkScarabTile`.
    pub item: Option<u64>,
    /// Target Item/Container userdata id when `target_is_item_userdata`.
    pub target_item: Option<u64>,
    pub target_creature: Option<u64>,
    /// `true` only for Item/Container userdata (`destroyItem` type check).
    pub target_is_item_userdata: bool,
    /// `target.itemid` — `None` when the field is Lua nil (creature / missing).
    pub target_itemid: Option<u16>,
    pub target_actionid: u16,
    pub from: (u16, u16, u8),
    pub to: (u16, u16, u8),
    pub quest: Option<QuestChestSpec>,
}

/// Arguments for [`LuaMutation::ConjureItem`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConjureRequest {
    pub player: u64,
    /// Extra dual-hand mana (integer first arg, or `Spell.mana`).
    pub mana_cost: i32,
    pub reagent_id: u16,
    pub conjure_id: u16,
    /// `None` / `Some(0)` trigger `ItemType:getCharges()` fallback.
    pub conjure_count: Option<u32>,
    pub effect: u8,
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
    static MUTATION_I32_RESULT: Cell<Option<i32>> = const { Cell::new(None) };
}

// Per-test override — `OnceLock` cannot replace an applier, and tests in this
// crate run in parallel. Production still uses `MUTATION_APPLIER`.
#[cfg(test)]
thread_local! {
    static TEST_APPLIER: Cell<Option<LuaMutationApplier>> = const { Cell::new(None) };
}

/// Register the core handler that applies mutations (called once at startup).
pub fn register_lua_mutation_applier(applier: LuaMutationApplier) {
    #[cfg(test)]
    TEST_APPLIER.with(|c| c.set(Some(applier)));
    let _ = MUTATION_APPLIER.set(applier);
}

/// Execute `f` with an active Lua mutation scope bound to `world`.
///
/// Nesting is supported: inventory equip/deequip hooks may enter a new scope while
/// an outer `onCastSpell` / talkaction scope is still active. Clearing to null on
/// exit would break subsequent `addItem` in the same Lua callback.
pub fn with_lua_mutation_scope<F, R>(world: *mut (), f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev_world = MUTATION_WORLD.replace(world);
    let prev_bool = MUTATION_BOOL_RESULT.replace(None);
    let prev_item = MUTATION_ITEM_RESULT.replace(None);
    let prev_i32 = MUTATION_I32_RESULT.replace(None);
    let result = f();
    MUTATION_WORLD.set(prev_world);
    MUTATION_BOOL_RESULT.set(prev_bool);
    MUTATION_ITEM_RESULT.set(prev_item);
    MUTATION_I32_RESULT.set(prev_i32);
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

pub fn take_mutation_i32_result() -> Option<i32> {
    MUTATION_I32_RESULT.take()
}

pub fn set_mutation_i32_result(v: i32) {
    MUTATION_I32_RESULT.set(Some(v));
}

fn apply_mutation(mutation: LuaMutation) -> Result<(), String> {
    let world = MUTATION_WORLD.get();
    if world.is_null() {
        return Err("Lua mutation scope not active".into());
    }
    #[cfg(test)]
    if let Some(applier) = TEST_APPLIER.with(|c| c.get()) {
        return applier(world, mutation);
    }
    let applier = MUTATION_APPLIER
        .get()
        .ok_or_else(|| "Lua mutation applier not registered".to_string())?;
    applier(world, mutation)
}

#[cfg(test)]
mod mutation_scope_tests {
    use super::{MUTATION_WORLD, with_lua_mutation_scope};

    #[test]
    fn nested_mutation_scope_restores_outer_world() {
        // Inventory hooks nest `with_lua_mutation_scope` inside `onCastSpell`;
        // clearing to null on inner exit broke subsequent `addItem`.
        let outer = 0x1 as *mut ();
        let inner = 0x2 as *mut ();
        with_lua_mutation_scope(outer, || {
            assert_eq!(MUTATION_WORLD.get(), outer);
            with_lua_mutation_scope(inner, || {
                assert_eq!(MUTATION_WORLD.get(), inner);
            });
            assert_eq!(MUTATION_WORLD.get(), outer);
        });
        assert!(MUTATION_WORLD.get().is_null());
    }
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
) -> Result<bool, String> {
    apply_mutation(LuaMutation::PlayerRemoveItem {
        creature_id,
        item_type,
        count,
        sub_type,
        ignore_equipped,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
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

/// `player:getDepotLocker(depotId)` — TFS `Player::getDepotLocker` (`player.cpp:826`).
pub fn call_lua_get_depot_locker(creature_id: u64, depot_id: u32) -> Result<Option<u64>, String> {
    apply_mutation(LuaMutation::PlayerGetDepotLocker {
        creature_id,
        depot_id,
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

pub fn call_lua_set_custom_attribute(item_id: u64, key: String, value: i64) -> Result<(), String> {
    apply_mutation(LuaMutation::ItemSetCustomAttribute {
        item_id,
        key,
        value,
    })
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
pub fn call_lua_add_condition(creature_id: u64, spec: ConditionApplySpec) -> Result<(), String> {
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

/// `creature:addHealth(healthChange)` — E4. Clamps HP to `[0, max]`.
/// Domain: TFS `luaCreatureAddHealth`. Outcomes: 772 `Heal` (`magic.cc:2086`).
pub fn call_lua_add_health(creature_id: u64, health_change: i32) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerAddHealth {
        creature_id,
        health_change,
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
pub fn call_lua_item_transform(item_id: u64, new_type: u16, sub_type: i32) -> Result<bool, String> {
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

/// `Game.startRaid(name)` — TFS `luaGameStartRaid`. Returns `RETURNVALUE_*` integer.
pub fn call_start_raid(name: String) -> Result<i32, String> {
    apply_mutation(LuaMutation::StartRaid { name })?;
    Ok(take_mutation_i32_result().unwrap_or(61))
}

pub fn call_create_npc(
    name: String,
    x: u16,
    y: u16,
    z: u8,
    force: bool,
) -> Result<Option<u64>, String> {
    apply_mutation(LuaMutation::CreateNpc {
        name,
        x,
        y,
        z,
        force,
    })?;
    Ok(take_mutation_item_result())
}

pub fn call_npc_set_master_pos(
    npc_id: u64,
    x: u16,
    y: u16,
    z: u8,
    radius: Option<u16>,
) -> Result<bool, String> {
    apply_mutation(LuaMutation::NpcSetMasterPos {
        npc_id,
        x,
        y,
        z,
        radius,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

pub fn call_set_game_state(state: i32) -> Result<(), String> {
    apply_mutation(LuaMutation::SetGameState { state })
}

pub fn call_game_reload(reload_type: i32) -> Result<(), String> {
    apply_mutation(LuaMutation::GameReload { reload_type })
}

pub fn call_lua_set_sex(creature_id: u64, sex: u8) -> Result<bool, String> {
    apply_mutation(LuaMutation::PlayerSetSex { creature_id, sex })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

pub fn call_party_set_shared_experience(party_id: u32, enabled: bool) -> Result<bool, String> {
    apply_mutation(LuaMutation::PartySetSharedExperience { party_id, enabled })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
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

pub fn call_send_text_message(creature_id: u64, msg_class: u8, text: String) -> Result<(), String> {
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

/// `player:removeMoney(amount)` — returns whether inventory coins covered `amount`.
pub fn call_lua_remove_money(creature_id: u64, amount: u64) -> Result<bool, String> {
    apply_mutation(LuaMutation::PlayerRemoveMoney {
        creature_id,
        amount,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// `player:setBankBalance(balance)`.
pub fn call_lua_set_bank_balance(creature_id: u64, balance: u64) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerSetBankBalance {
        creature_id,
        balance,
    })
}

pub fn call_lua_set_premium_ends_at(creature_id: u64, ends_at: u32) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerSetPremiumEndsAt {
        creature_id,
        ends_at,
    })
}

pub fn call_lua_set_storage_value(creature_id: u64, key: u32, value: i32) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerSetStorageValue {
        creature_id,
        key,
        value,
    })
}

/// `player:setTown(town)` — `luaPlayerSetTown`. Assigns `town_id` only.
pub fn call_lua_set_town(creature_id: u64, town_id: u32) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerSetTown {
        creature_id,
        town_id,
    })
}

/// `setWorldLight(level, color)` — TFS `LuaScriptInterface::luaSetWorldLight`.
/// C++ ref: `gameserver/src/luascript.cpp:3132-3145`.
pub fn call_lua_set_world_light(level: u8, color: u8) -> Result<bool, String> {
    apply_mutation(LuaMutation::SetWorldLight { level, color })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// `Game.clearField(item[, excludeCreature])` — 772 `ClearField` (`moveuse.cc:569`).
pub fn call_clear_field(
    exclude_item_id: u64,
    exclude_creature_id: Option<u64>,
) -> Result<(), String> {
    apply_mutation(LuaMutation::ClearField {
        exclude_item_id,
        exclude_creature_id,
    })
}

/// `player:addSkillTries(skill, tries)` — `luaPlayerAddSkillTries`.
/// `true` when the player exists, `None` → Lua `nil`.
pub fn call_lua_add_skill_tries(
    creature_id: u64,
    skill: i32,
    tries: u64,
) -> Result<Option<bool>, String> {
    apply_mutation(LuaMutation::PlayerAddSkillTries {
        creature_id,
        skill,
        tries,
    })?;
    Ok(take_mutation_bool_result())
}

/// `creature:say(text[, type])` — E5. Viewport say; no spell parser.
pub fn call_lua_player_say(creature_id: u64, text: String, speak_type: u8) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerSay {
        creature_id,
        text,
        speak_type,
    })
}

/// `player:showTextDialog(itemId, text)` — E6. Read-only `0x96` window.
pub fn call_lua_show_text_dialog(
    creature_id: u64,
    item_type: u16,
    text: String,
) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerShowTextDialog {
        creature_id,
        item_type,
        text,
    })
}

/// `tile:addItem(itemId[, count[, flags]])` — `luaTileAddItem`.
pub fn call_lua_tile_add_item(
    x: u16,
    y: u16,
    z: u8,
    item_type: u16,
    count: u16,
    flags: u32,
) -> Result<Option<u64>, String> {
    apply_mutation(LuaMutation::TileAddItem {
        x,
        y,
        z,
        item_type,
        count,
        flags,
    })?;
    Ok(take_mutation_item_result())
}

/// `Game.createItem(itemId[, count[, position]])` — `luaGameCreateItem`.
pub fn call_lua_game_create_item(
    item_type: u16,
    count: u16,
    position: Option<(u16, u16, u8)>,
) -> Result<Option<u64>, String> {
    apply_mutation(LuaMutation::GameCreateItem {
        item_type,
        count,
        position,
    })?;
    Ok(take_mutation_item_result())
}

/// `player:addItemEx` / `container:addItemEx` / `tile:addItemEx`.
/// Returns `RETURNVALUE_*` (`0` = `RETURNVALUE_NOERROR`).
pub fn call_lua_add_item_ex(
    item_id: u64,
    dest: LuaMoveDestination,
    can_drop_on_map: bool,
    index: i32,
    flags: u32,
) -> Result<i32, String> {
    apply_mutation(LuaMutation::AddItemEx {
        item_id,
        dest,
        can_drop_on_map,
        index,
        flags,
    })?;
    Ok(take_mutation_i32_result().unwrap_or(1))
}

/// `Game.createTile(position[, isDynamic])` — `luaGameCreateTile`.
pub fn call_lua_game_create_tile(x: u16, y: u16, z: u8, is_dynamic: bool) -> Result<(), String> {
    apply_mutation(LuaMutation::GameCreateTile {
        x,
        y,
        z,
        is_dynamic,
    })
}

/// `doTargetCombat` / `doTargetCombatHealth` — `luaDoTargetCombat`.
pub fn call_do_target_combat_health(
    attacker_id: Option<u64>,
    target_id: u64,
    combat_type: i32,
    damage_min: i32,
    damage_max: i32,
    effect: i32,
) -> Result<bool, String> {
    apply_mutation(LuaMutation::TargetCombatHealth {
        attacker_id,
        target_id,
        combat_type,
        damage_min,
        damage_max,
        effect,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// `player:registerEvent` / `unregisterEvent` — TFS `luaPlayerRegisterEvent`.
pub fn call_player_register_creature_event(
    creature_id: u64,
    name: String,
    register: bool,
) -> Result<bool, String> {
    apply_mutation(LuaMutation::PlayerRegisterCreatureEvent {
        creature_id,
        name,
        register,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// `player:sendOutfitWindow()` — TFS `luaPlayerSendOutfitWindow`.
pub fn call_lua_send_outfit_window(creature_id: u64) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerSendOutfitWindow { creature_id })
}

/// `player:setOutfit(...)` — TFS `luaPlayerSetOutfit`.
pub fn call_lua_set_outfit(
    creature_id: u64,
    look_type: i32,
    look_head: i32,
    look_body: i32,
    look_legs: i32,
    look_feet: i32,
    look_addons: i32,
) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerSetOutfit {
        creature_id,
        look_type,
        look_head,
        look_body,
        look_legs,
        look_feet,
        look_addons,
    })
}

/// `player:setVocation(id)` — TFS `luaPlayerSetVocation`.
pub fn call_lua_set_vocation(creature_id: u64, vocation_id: i32) -> Result<bool, String> {
    apply_mutation(LuaMutation::PlayerSetVocation {
        creature_id,
        vocation_id,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// `creature:setDirection(dir)` — TFS `luaCreatureSetDirection`.
pub fn call_lua_set_direction(creature_id: u64, direction: u8) -> Result<bool, String> {
    apply_mutation(LuaMutation::PlayerSetDirection {
        creature_id,
        direction,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// `saveServer()` / `Game.saveServer()` — TFS `luaSaveServer` → `Game::saveGameState`.
pub fn call_lua_save_server() -> Result<(), String> {
    apply_mutation(LuaMutation::SaveServer)
}

/// `player:setGhostMode(enabled)` — TFS `luaPlayerSetGhostMode`.
pub fn call_lua_set_ghost_mode(creature_id: u64, enabled: bool) -> Result<(), String> {
    apply_mutation(LuaMutation::PlayerSetGhostMode {
        creature_id,
        enabled,
    })
}

/// `creature:remove()` — TFS `luaCreatureRemove`.
pub fn call_lua_creature_remove(creature_id: u64) -> Result<(), String> {
    apply_mutation(LuaMutation::CreatureRemove { creature_id })
}

/// `house:setOwnerGuid(guid)`.
pub fn call_house_set_owner(house_id: u32, guid: u32) -> Result<(), String> {
    apply_mutation(LuaMutation::HouseSetOwner { house_id, guid })
}

/// `house:setAccessList(listId, text)`.
pub fn call_house_set_access_list(house_id: u32, list_id: u32, text: String) -> Result<(), String> {
    apply_mutation(LuaMutation::HouseSetAccessList {
        house_id,
        list_id,
        text,
    })
}

/// `house:kickPlayer(player, target)`.
pub fn call_house_kick_player(
    house_id: u32,
    kicker_id: u64,
    target_id: u64,
) -> Result<bool, String> {
    apply_mutation(LuaMutation::HouseKickPlayer {
        house_id,
        kicker_id,
        target_id,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// `house:save()`.
pub fn call_house_save(house_id: u32) -> Result<(), String> {
    apply_mutation(LuaMutation::HouseSave { house_id })
}

/// `player:setEditHouse(house, listId)`.
pub fn call_player_set_edit_house(
    creature_id: u64,
    house_id: u32,
    list_id: u32,
) -> Result<bool, String> {
    apply_mutation(LuaMutation::PlayerSetEditHouse {
        creature_id,
        house_id,
        list_id,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// `player:sendHouseWindow(house, listId)`.
pub fn call_player_send_house_window(
    creature_id: u64,
    house_id: u32,
    list_id: u32,
) -> Result<bool, String> {
    apply_mutation(LuaMutation::PlayerSendHouseWindow {
        creature_id,
        house_id,
        list_id,
    })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// Native `onUse*` / `destroyItem` / `onUseQuest` / `checkScarabTile`.
pub fn call_lua_tool_use(request: ToolUseRequest) -> Result<bool, String> {
    apply_mutation(LuaMutation::ToolUse { request })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}

/// Native `Player:conjureItem`.
pub fn call_lua_conjure_item(request: ConjureRequest) -> Result<bool, String> {
    apply_mutation(LuaMutation::ConjureItem { request })?;
    Ok(take_mutation_bool_result().unwrap_or(false))
}
