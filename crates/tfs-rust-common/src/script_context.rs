//! Script read context trait — shared by core and lua crates (no circular dependency).
//!
//! C++ reference: `LuaScriptInterface` read accessors resolving userdata IDs to game objects.

use crate::Position;

/// Lua / script creature handle (`slotmap` key bits as u64).
pub type ScriptCreatureId = u64;

/// Lua / script item handle (`slotmap` key bits as u64).
pub type ScriptItemId = u64;

/// Creature fields for script read APIs (`Creature:getName`, …).
#[derive(Clone, Debug)]
pub struct ScriptCreatureData {
    pub name: String,
    pub guid: u32,
}

/// Item fields for script read APIs (`Item:getId` / `getType` / …).
#[derive(Clone, Debug)]
pub struct ScriptItemData {
    pub item_type: u16,
    pub count: u16,
    pub weight: u32,
    pub name: String,
    pub action_id: u16,
    pub unique_id: u32,
    pub is_store_item: bool,
}

/// ID handle wrapper for creatures passed to Lua userdata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptCreatureRef(pub ScriptCreatureId);

/// ID handle wrapper for items passed to Lua userdata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptItemRef(pub ScriptItemId);

/// Cylinder reference for `Item:getParent` / `getTopParent` — `luascript.cpp` `pushCylinder`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptCylinder {
    Player(ScriptCreatureId),
    Container(ScriptItemId),
    Tile(Position),
}

/// Container read snapshot for Lua `Container:*` methods.
#[derive(Clone, Debug)]
pub struct ScriptContainerData {
    pub size: u32,
    pub capacity: u32,
    pub empty_slots: u32,
    pub item_holding_count: u32,
    pub corpse_owner: u32,
}

/// Read-only game object resolution during script execution.
///
/// Implemented by `GameWorld` in `tfs-rust-core`. Lua userdata resolves handles via
/// thread-local scope set by `tfs-rust-lua::with_lua_context`.
pub trait ScriptContext {
    fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData>;
    fn get_item(&self, id: ScriptItemId) -> Option<ScriptItemRef>;
    fn get_config_string(&self, key: &str) -> Option<String>;
    /// `Player` equipment slot — raw item id (`slotmap` key bits as u64).
    fn get_player_slot_item_id(
        &self,
        creature_id: ScriptCreatureId,
        slot: u8,
    ) -> Option<ScriptItemId> {
        let _ = (creature_id, slot);
        None
    }
    fn get_player_capacity(&self, creature_id: ScriptCreatureId) -> Option<u32> {
        let _ = creature_id;
        None
    }
    fn get_player_free_capacity(&self, creature_id: ScriptCreatureId) -> Option<u32> {
        let _ = creature_id;
        None
    }
    fn get_player_item_type_count(
        &self,
        creature_id: ScriptCreatureId,
        item_id: u16,
        sub_type: i32,
    ) -> Option<u32> {
        let _ = (creature_id, item_id, sub_type);
        None
    }
    fn get_item_data(&self, id: ScriptItemId) -> Option<ScriptItemData> {
        let _ = id;
        None
    }

    /// Resolve server item type id from name — `Item::items.getItemIdByName`.
    fn get_item_type_id_by_name(&self, name: &str) -> Option<u16> {
        let _ = name;
        None
    }

    /// TFS `player:getItemById` — `Game::findItemOfType(player, …)`.
    fn find_player_item_by_type(
        &self,
        creature_id: ScriptCreatureId,
        item_id: u16,
        depth_search: bool,
        sub_type: i32,
    ) -> Option<ScriptItemId> {
        let _ = (creature_id, item_id, depth_search, sub_type);
        None
    }

    fn is_registered_container(&self, item_id: ScriptItemId) -> bool {
        let _ = item_id;
        false
    }

    fn get_container_data(&self, item_id: ScriptItemId) -> Option<ScriptContainerData> {
        let _ = item_id;
        None
    }

    fn get_container_item_at(
        &self,
        container_id: ScriptItemId,
        index: u32,
    ) -> Option<ScriptItemId> {
        let _ = (container_id, index);
        None
    }

    fn get_container_items(&self, container_id: ScriptItemId) -> Vec<ScriptItemId> {
        let _ = container_id;
        Vec::new()
    }

    fn container_has_item(&self, container_id: ScriptItemId, item_id: ScriptItemId) -> bool {
        let _ = (container_id, item_id);
        false
    }

    fn get_container_item_count_by_id(
        &self,
        container_id: ScriptItemId,
        item_type: u16,
        sub_type: i32,
    ) -> u32 {
        let _ = (container_id, item_type, sub_type);
        0
    }

    fn get_player_container_id(
        &self,
        creature_id: ScriptCreatureId,
        container_id: ScriptItemId,
    ) -> Option<u8> {
        let _ = (creature_id, container_id);
        None
    }

    fn get_player_container_by_cid(
        &self,
        creature_id: ScriptCreatureId,
        client_cid: u8,
    ) -> Option<ScriptItemId> {
        let _ = (creature_id, client_cid);
        None
    }

    fn get_player_container_index(
        &self,
        creature_id: ScriptCreatureId,
        client_cid: u8,
    ) -> Option<u16> {
        let _ = (creature_id, client_cid);
        None
    }

    fn get_item_parent(&self, item_id: ScriptItemId) -> Option<ScriptCylinder> {
        let _ = item_id;
        None
    }

    fn get_item_top_parent(&self, item_id: ScriptItemId) -> Option<ScriptCylinder> {
        let _ = item_id;
        None
    }

    fn get_item_position(&self, item_id: ScriptItemId) -> Option<Position> {
        let _ = item_id;
        None
    }

    /// 772 `SKILL_FED` `Cycle` — food-remaining rounds (`crskill.cc:220`).
    /// Used by `player:getFood()` in Lua.
    fn get_player_food(&self, creature_id: ScriptCreatureId) -> Option<u32> {
        let _ = creature_id;
        None
    }

    /// `player:getLevel()` — `Player::getLevel` (`player.h`). LUA-2 read used by
    /// channel `onSpeak` gating (e.g. advertising.lua level-1 cancel).
    fn get_player_level(&self, creature_id: ScriptCreatureId) -> Option<i32> {
        let _ = creature_id;
        None
    }

    /// `player:getAccountType()` — `accounts.type` tier (`enums.h:80-85`,
    /// `ACCOUNT_TYPE_NORMAL=1` … `ACCOUNT_TYPE_GOD=6`). LUA-2 read; the backing
    /// field is plumbed from `accounts.type` at login (`iologindata.cpp`).
    fn get_player_account_type(&self, creature_id: ScriptCreatureId) -> Option<u8> {
        let _ = creature_id;
        None
    }

    /// `player:getVocation():getId()` backing read — `players.vocation`
    /// (`player.h` `Vocation`). LUA-2 returns the raw vocation id; the Lua
    /// `Vocation` userdata wraps it (§1.4 option a).
    fn get_player_vocation_id(&self, creature_id: ScriptCreatureId) -> Option<i32> {
        let _ = creature_id;
        None
    }

    /// `player:hasFlag(flag)` — `Player::hasFlag` (`player.h`) over the resolved
    /// `groups.xml` flag bits for `players.group_id`. LUA-2 read; defaults to
    /// `false` so `NullEventDispatcher`/tests need no change.
    fn player_has_flag(&self, creature_id: ScriptCreatureId, flag: u64) -> bool {
        let _ = (creature_id, flag);
        false
    }

    /// `player:getCondition(type, id, subId)` — `luascript.cpp:2116`
    /// `Creature::getCondition`. LUA-4 read; returns remaining ticks if an
    /// active condition matching `(ctype, cond_id, sub_id)` exists, `None`
    /// otherwise. `ctype` is the Lua-facing 772 bit-flag value; the `GameWorld`
    /// impl maps it to the Rust `ConditionType` enum. Defaults to `None`.
    fn get_creature_condition(
        &self,
        creature_id: ScriptCreatureId,
        ctype: i32,
        cond_id: i32,
        sub_id: u32,
    ) -> Option<i32> {
        let _ = (creature_id, ctype, cond_id, sub_id);
        None
    }

    /// `Player(name)` constructor — `luascript.cpp` `luaPlayerCreate`. LUA-4
    /// read; resolves an online player by name to their `ScriptCreatureId`.
    /// Defaults to `None` (player not found → Lua `nil`).
    fn get_player_by_name(&self, name: &str) -> Option<ScriptCreatureId> {
        let _ = name;
        None
    }

    /// `player:getGroup():getId()` backing read — `players.group_id`
    /// (`player.h` `Group`). CH-6 talkaction access gating; defaults to
    /// `None` (player not found).
    fn get_player_group_id(&self, creature_id: ScriptCreatureId) -> Option<u16> {
        let _ = creature_id;
        None
    }

    /// `group:getAccess()` backing read — `groups.xml` `access` flag for the
    /// given group id (`src/groups.cpp`). CH-6 talkaction access gating;
    /// defaults to `false`.
    fn get_group_access(&self, group_id: u16) -> bool {
        let _ = group_id;
        false
    }

    /// `player:getPosition()` backing read — `Creature::getPosition`
    /// (`creature.h`). CH-6 talkaction `sendMagicEffect` at player position;
    /// defaults to `None`.
    fn get_player_position(&self, creature_id: ScriptCreatureId) -> Option<Position> {
        let _ = creature_id;
        None
    }

    /// `player:getDirection()` backing read — `Creature::getDirection`
    /// (`creature.h`). PC-3a: spell variant construction offsets the center
    /// position by one tile in the player's facing direction when
    /// `needDirection(true)` is set (beam/wave/strike spells).
    /// Returns `None` if the creature is not found; defaults to `None`.
    fn get_player_direction(&self, creature_id: ScriptCreatureId) -> Option<u8> {
        let _ = creature_id;
        None
    }

    /// `ItemType:isStackable()` backing read — `ItemType::stackable`
    /// (`src/items.h`). CH-6 talkaction `/i` count clamping; defaults to
    /// `false`.
    fn get_item_type_is_stackable(&self, item_type: u16) -> bool {
        let _ = item_type;
        false
    }

    /// `ItemType:isFluidContainer()` backing read — `ItemType::isFluidContainer`
    /// (`src/items.h`). CH-6 talkaction `/i` count clamping; defaults to
    /// `false`.
    fn get_item_type_is_fluid_container(&self, item_type: u16) -> bool {
        let _ = item_type;
        false
    }

    /// `player:getMagicLevel()` — `Player::getMagicLevel` (`player.h`).
    /// PC-3a Phase 1: value-callback spells (`computeDamage` / `computeHealing`)
    /// call `self:getMagicLevel()` inside `functions.lua`. Defaults to `None`.
    fn get_player_magic_level(&self, creature_id: ScriptCreatureId) -> Option<i32> {
        let _ = creature_id;
        None
    }

    /// Weapon combat parameters for the SKILL value callback
    /// (`CALLBACK_PARAM_SKILLVALUE`). C++ `Combat::getCombatDamage` —
    /// `combat.cpp:1155-1163` reads `player->getWeaponSkill()`,
    /// `player->getWeapon()->getAttack()`, `player->getAttackFactor()`.
    /// Returns `(skill, weapon_attack, attack_factor)`; defaults to
    /// `(0, 0, 1.0)` (fist skill, no weapon, normal factor).
    fn get_player_weapon_combat_params(&self, creature_id: ScriptCreatureId) -> WeaponCombatParams {
        let _ = creature_id;
        WeaponCombatParams::default()
    }

    /// Creatures standing on the given area offsets around `(cx,cy,cz)`.
    /// PC-3a Phase 3: `combat:getTargets` for `poison_storm.lua`.
    fn get_creatures_on_area(
        &self,
        center_x: u16,
        center_y: u16,
        center_z: u8,
        offsets: &[(i32, i32)],
    ) -> Vec<ScriptCreatureId> {
        let _ = (center_x, center_y, center_z, offsets);
        Vec::new()
    }

    /// `creature:isPlayer()` — true when the id resolves to a player.
    fn is_creature_player(&self, creature_id: ScriptCreatureId) -> bool {
        let _ = creature_id;
        false
    }
}

/// Weapon-derived inputs for the SKILL value callback (`combat.cpp:1155-1163`).
/// Returned by `ScriptContext::get_player_weapon_combat_params`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeaponCombatParams {
    /// `Player::getWeaponSkill` — skill level matching the equipped weapon type.
    pub skill: i32,
    /// `Item::getAttack` — equipped weapon attack value.
    pub attack: i32,
    /// `Player::getAttackFactor` — attack factor (1.0 = normal).
    pub attack_factor: f64,
}
