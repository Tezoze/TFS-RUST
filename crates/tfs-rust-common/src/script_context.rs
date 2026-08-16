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
    /// `item:getFluidType()` — `Item::getFluidType` (`item.h`).
    pub fluid_type: u16,
    /// TFS `Item::getSubType` / compat `item.type` (`item.cpp`, `compat.lua` ItemIndex).
    pub sub_type: u16,
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

/// Remere OTBM door/key custom-attr keys (`item_blob` / Lua `ITEM_ATTRIBUTE_*` string aliases).
/// TVP stores these as int attrs; TFS 1.4.2 bitflags collide, so map load keeps them as custom.
pub mod remere_attr {
    pub const KEYNUMBER: &str = "keynumber";
    pub const KEYHOLENUMBER: &str = "keyholenumber";
    pub const DOORQUESTNUMBER: &str = "doorquestnumber";
    pub const DOORQUESTVALUE: &str = "doorquestvalue";
    pub const DOORLEVEL: &str = "doorlevel";
    pub const CHESTQUESTNUMBER: &str = "chestquestnumber";
}

/// `Tile:getTopVisibleThing` result — Item or Creature userdata (`luascript.cpp`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptThing {
    Item(ScriptItemId),
    Creature(ScriptCreatureId),
}

/// Value returned by `item:getAttribute` / custom-attr reads.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptAttrValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
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

    /// `player:getEffectiveSkillLevel(skill)` — `luaPlayerGetEffectiveSkillLevel`
    /// → `Player::getSkillLevel` (`player.h`). `skill` is TFS `skills_t` (0–6).
    /// `None` when the creature is not a player or `skill` is out of range.
    fn get_player_effective_skill(&self, creature_id: ScriptCreatureId, skill: i32) -> Option<i32> {
        let _ = (creature_id, skill);
        None
    }

    /// `player:isPzLocked()` — TFS `Player::isPzLocked` (`player.h`).
    /// 772 outcome: `earliest_protection_zone_round > round_nr`.
    /// `None` when the creature is not a player (Lua `nil`).
    fn player_is_pz_locked(&self, creature_id: ScriptCreatureId) -> Option<bool> {
        let _ = creature_id;
        None
    }

    /// `player:getMurderTimestamps()` — 772 murder ring / TVP `Player::murderTimeStamps`.
    fn get_player_murder_timestamps(&self, creature_id: ScriptCreatureId) -> Vec<i64> {
        let _ = creature_id;
        Vec::new()
    }

    /// `player:getPlayerKillerEnd()` — unix `PlayerkillerEnd` / `skulltime`.
    fn get_player_killer_end(&self, creature_id: ScriptCreatureId) -> Option<i64> {
        let _ = creature_id;
        None
    }

    /// `player:getStorageValue(key)` — `Player::getStorageValue` (`player.cpp`).
    /// Missing key → `-1` (TFS / 772 quest empty). Defaults to `-1`.
    fn get_player_storage_value(&self, creature_id: ScriptCreatureId, key: u32) -> i32 {
        let _ = (creature_id, key);
        -1
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

    /// `player:getTown()` backing read — `players.town_id` (`player.h`).
    /// Defaults to `None` (not a player).
    fn get_player_town_id(&self, creature_id: ScriptCreatureId) -> Option<i32> {
        let _ = creature_id;
        None
    }

    /// `Town(id)` / `town:getTemplePosition()` — OTBM `TownData` by id.
    fn get_town_by_id(&self, town_id: u32) -> Option<ScriptTownData> {
        let _ = town_id;
        None
    }

    /// `Town(name)` — case-insensitive name match (`Towns::getTown`).
    fn get_town_by_name(&self, name: &str) -> Option<ScriptTownData> {
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

    /// `group:getMaxDepotItems()` — `Group::maxDepotItems` (`luascript.cpp:11459`).
    fn get_group_max_depot_items(&self, group_id: u16) -> u32 {
        let _ = group_id;
        2000
    }

    /// `group:getMaxVipEntries()` — `Group::maxVipEntries` (`luascript.cpp:11471`).
    fn get_group_max_vip_entries(&self, group_id: u16) -> u32 {
        let _ = group_id;
        20
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

    /// `ItemType:getCharges()` — `ItemType::charges` (`src/items.h`).
    /// PC-3a Phase 5: `Player:conjureItem` falls back to charges when count
    /// is omitted. Defaults to `0`.
    fn get_item_type_charges(&self, item_type: u16) -> u32 {
        let _ = item_type;
        0
    }

    /// `ItemType:getDestroyId()` — `ItemType::destroyTo` (`src/items.h`).
    /// XML `destroyto`; 772 `DESTROYTARGET`. Defaults to `0`.
    fn get_item_type_destroy_id(&self, item_type: u16) -> u16 {
        let _ = item_type;
        0
    }

    /// `ItemType:getFluidSource()` — `ItemType::fluidSource` (`src/items.h`).
    /// XML `fluidsource` as 772 sequential `FLUID_*`. Defaults to `0` (`FLUID_NONE`).
    fn get_item_type_fluid_source(&self, item_type: u16) -> u8 {
        let _ = item_type;
        0
    }

    /// `ItemType:getName()` — `ItemType::name` (`src/items.h`).
    /// XML `name`; empty when the type is unknown. Defaults to `""`.
    fn get_item_type_name(&self, item_type: u16) -> String {
        let _ = item_type;
        String::new()
    }

    /// `ItemType:getArticle()` — `ItemType::article` (`src/items.h`).
    /// XML `article`; empty when unset. Defaults to `""`.
    fn get_item_type_article(&self, item_type: u16) -> String {
        let _ = item_type;
        String::new()
    }

    /// `ItemType:getPluralName()` — `ItemType::getPluralName` (`src/items.h`).
    /// XML `plural`, else `name` / `name+"s"`. Defaults to `""`.
    fn get_item_type_plural_name(&self, item_type: u16) -> String {
        let _ = item_type;
        String::new()
    }

    /// `ItemType:getWeight([count])` type weight — `ItemType::weight` (`src/items.h`).
    /// Lua multiplies by `max(1, count)`. Defaults to `0`.
    fn get_item_type_weight(&self, item_type: u16) -> u32 {
        let _ = item_type;
        0
    }

    /// `ItemType:isContainer()` — `ItemType::isContainer` (`src/items.h`).
    /// `group == ITEM_GROUP_CONTAINER`. Defaults to `false`.
    fn get_item_type_is_container(&self, item_type: u16) -> bool {
        let _ = item_type;
        false
    }

    /// `item:hasAttribute(key)` — `ItemAttributes::hasAttribute`
    /// (`src/item.h`). `attr_bits` is a Lua `itemAttrTypes` bitflag
    /// (`ITEM_ATTRIBUTE_*`). Defaults to `false`.
    fn item_has_attribute(&self, item_id: ScriptItemId, attr_bits: u32) -> bool {
        let _ = (item_id, attr_bits);
        false
    }

    /// `item:hasAttribute("keynumber")` — Remere custom-attr presence.
    fn item_has_custom_attribute(&self, item_id: ScriptItemId, key: &str) -> bool {
        let _ = (item_id, key);
        false
    }

    /// `item:getAttribute` for bitflag int attrs (`ITEM_ATTRIBUTE_ACTIONID`, …).
    /// Missing int attrs return `0` like TFS `getIntAttr`.
    fn item_get_int_attribute(&self, item_id: ScriptItemId, attr_bits: u32) -> Option<i64> {
        let _ = (item_id, attr_bits);
        None
    }

    /// `item:getAttribute` / `getCustomAttribute` for string keys (Remere door/key).
    fn item_get_custom_attribute(
        &self,
        item_id: ScriptItemId,
        key: &str,
    ) -> Option<ScriptAttrValue> {
        let _ = (item_id, key);
        None
    }

    /// `tile:getTopVisibleThing([creature])` — `Tile::getTopVisibleThing` (`tile.cpp`).
    /// `viewer == None` matches C++ nullptr (skip invisible/ghost only).
    fn tile_get_top_visible_thing(
        &self,
        x: u16,
        y: u16,
        z: u8,
        viewer: Option<ScriptCreatureId>,
    ) -> Option<ScriptThing> {
        let _ = (x, y, z, viewer);
        None
    }

    /// `group:hasFlag(flag)` — `Group::flags & flag` (`src/groups.cpp`).
    /// PC-3a Phase 5: `conjureItem` dual-hand mana gate uses
    /// `PlayerFlag_HasInfiniteMana`. Defaults to `false`.
    fn group_has_flag(&self, group_id: u16, flag: u64) -> bool {
        let _ = (group_id, flag);
        false
    }

    /// `player:getMana()` — `Player::getMana` (`player.h`).
    /// PC-3a Phase 5: `conjureItem` dual-hand second-conjure mana check.
    /// Defaults to `None`.
    fn get_player_mana(&self, creature_id: ScriptCreatureId) -> Option<i32> {
        let _ = creature_id;
        None
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

    /// `COMBAT_FORMULA_SKILL` damage bounds — TFS API shape, era formula.
    ///
    /// Default (no `GameWorld`) uses **772 ClassicProbe ceiling** for deterministic
    /// unit tests. Live `GameWorld` overrides and rolls one ProbeValue like
    /// `GetAttackDamage` (returns `(v, v)`).
    fn get_formula_skill_damage_bounds(
        &self,
        creature_id: ScriptCreatureId,
        _min_a: f64,
        min_b: f64,
        max_a: f64,
        max_b: f64,
    ) -> (i32, i32) {
        let p = self.get_player_weapon_combat_params(creature_id);
        // 772 `ProbeValue` ceiling: `(randomMax * attack * (skill*skillMult + skillBase)) / 10000`
        // balanced fight mode. Tunables match `MechanicsProfile::for_version(772)`.
        let attack = p.attack.max(0);
        let skill = p.skill.max(0);
        let max_value = attack.saturating_mul(skill.saturating_mul(5).saturating_add(50));
        let weapon_max = (99i32.saturating_mul(max_value)) / 10000;
        let lo = min_b as i32;
        let hi = (f64::from(weapon_max) * max_a + max_b).round() as i32;
        (lo, hi.max(lo))
    }

    /// `formulas.spell.levelMult` / `magicMult` — defaults match 772 / 1098 (`2`, `3`).
    fn get_spell_coeff(&self) -> (i32, i32) {
        (2, 3)
    }

    /// `Player:computeDamage` magnitude range after level/magic formula (`magic.cc:776`).
    ///
    /// Returns positive `(lo, hi)` for `damage±variation`. Default uses [`get_spell_coeff`].
    /// `GameWorld` overrides with `MechanicsProfile` + Tier-2 `getSpellDamage`.
    fn compute_magic_damage_range(
        &self,
        creature_id: ScriptCreatureId,
        damage: i32,
        variation: i32,
        limit_minimum: bool,
        limit_maximum: bool,
    ) -> (i32, i32) {
        let level = self.get_player_level(creature_id).unwrap_or(0);
        let magic = self.get_player_magic_level(creature_id).unwrap_or(0);
        let (lm, mm) = self.get_spell_coeff();
        let mut formula = lm * level + mm * magic;
        // Match `functions.lua` / decompile flag clamps (`<=99` floor, `>=101` cap).
        if (limit_minimum && formula <= 99) || (limit_maximum && formula >= 101) {
            formula = 100;
        }
        let lo = ((damage - variation) * formula) / 100;
        let hi = ((damage + variation) * formula) / 100;
        (lo.min(hi), lo.max(hi))
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

    /// `creature:isInGhostMode()` — `Creature::isInGhostMode` (`creature.h`):
    /// `false` for non-players; `Player::isInGhostMode` returns `ghostMode`
    /// (`player.h:363`). Safe to call on any creature (`luascript.cpp:7515`).
    fn is_creature_in_ghost_mode(&self, creature_id: ScriptCreatureId) -> bool {
        let _ = creature_id;
        false
    }

    /// `item:getUniqueId()` fallback — TFS `ScriptEnvironment::addThing`
    /// (`luascript.cpp:110-134`): returns `ATTR_UNIQUE_ID` if set, otherwise
    /// registers the item in the script env local map with a generated UID > 65535.
    fn register_script_item_uid(&self, item_id: ScriptItemId) -> u32 {
        let _ = item_id;
        0
    }

    /// `getDepotId(uid)` — TFS `luaGetDepotId` (`luascript.cpp:3766`):
    /// looks up the item by UID (ATTR_UNIQUE_ID ≤ 65535 or local map UID > 65535)
    /// and returns its `ATTR_DEPOT_ID` (`DepotLocker::getDepotId`).
    fn get_depot_id_by_uid(&self, uid: u32) -> Option<u32> {
        let _ = uid;
        None
    }

    /// `Tile(pos)` existence — true when the map has a tile at `(x,y,z)`.
    fn tile_exists(&self, x: u16, y: u16, z: u8) -> bool {
        let _ = (x, y, z);
        false
    }

    /// `tile:hasProperty(CONST_PROP_*)` — `Tile::hasProperty` (`tile.cpp:27`).
    /// PC-3a Phase 8: field runes check `CONST_PROP_BLOCKSOLID` (0).
    fn tile_has_property(&self, x: u16, y: u16, z: u8, prop: i32) -> bool {
        let _ = (x, y, z, prop);
        false
    }

    /// `Game.getWorldType()` — `Game::getWorldType` (`game.h`).
    /// Returns Lua `WORLD_TYPE_*` ordinal (`0=nopvp, 1=pvp, 2=pvp-enforced`).
    fn get_world_type(&self) -> i32 {
        1 // WORLD_TYPE_PVP default
    }

    /// `MonsterType(name)` — lookType for outfit condition (`monsters.h`).
    fn get_monster_type_look_type(&self, name: &str) -> Option<i32> {
        let _ = name;
        None
    }

    /// `MonsterType:isIllusionable()` — `<flag illusionable=…>`.
    fn get_monster_type_is_illusionable(&self, name: &str) -> bool {
        let _ = name;
        false
    }

    /// `MonsterType(name)` exists in the loaded monster database.
    fn monster_type_exists(&self, name: &str) -> bool {
        let _ = name;
        false
    }

    /// `tile:hasFlag(TILESTATE_*)` — `Tile::hasFlag` (`tile.cpp`).
    fn tile_has_flag(&self, x: u16, y: u16, z: u8, flags: i32) -> bool {
        let _ = (x, y, z, flags);
        false
    }

    /// Ground **server item type** id (`TileBody.ground`) — map speed / flags.
    fn tile_get_ground_type(&self, x: u16, y: u16, z: u8) -> Option<u16> {
        let _ = (x, y, z);
        None
    }

    /// `tile:getGround()` — SlotMap ground item (`luaTileGetGround` returns Item userdata).
    fn tile_get_ground_item(&self, x: u16, y: u16, z: u8) -> Option<ScriptItemId> {
        let _ = (x, y, z);
        None
    }

    /// `tile:getTopDownItem()` — top of the down-item stack.
    fn tile_get_top_down_item(&self, x: u16, y: u16, z: u8) -> Option<ScriptItemId> {
        let _ = (x, y, z);
        None
    }

    /// `tile:getItems()` — top + down items (excluding ground).
    fn tile_get_items(&self, x: u16, y: u16, z: u8) -> Vec<ScriptItemId> {
        let _ = (x, y, z);
        Vec::new()
    }

    /// `tile:getCreatures()` — creatures standing on the tile (`luascript.cpp` `luaTileGetCreatures`).
    fn tile_get_creatures(&self, x: u16, y: u16, z: u8) -> Vec<ScriptCreatureId> {
        let _ = (x, y, z);
        Vec::new()
    }

    /// `tile:getBottomCreature()` — `luaTileGetBottomCreature` → TFS `creatures->rbegin()`.
    /// Rust `push`s newest last, so the oldest (first) creature is the bottom.
    fn tile_get_bottom_creature(&self, x: u16, y: u16, z: u8) -> Option<ScriptCreatureId> {
        self.tile_get_creatures(x, y, z).first().copied()
    }

    /// `tile:getCreatureCount()` — `Tile::getCreatureCount`.
    fn tile_get_creature_count(&self, x: u16, y: u16, z: u8) -> u32 {
        self.tile_get_creatures(x, y, z).len() as u32
    }

    /// `tile:getThingCount()` — ground + top + creatures + down (`tile.h`).
    fn tile_get_thing_count(&self, x: u16, y: u16, z: u8) -> u32 {
        let _ = (x, y, z);
        0
    }

    /// `tile:getThing(index)` — TFS stack order (`Tile::getThing`).
    fn tile_get_thing(&self, x: u16, y: u16, z: u8, index: u32) -> Option<ScriptThing> {
        let _ = (x, y, z, index);
        None
    }

    /// `tile:getItemById(itemId)` — first matching item including ground (`findItemOfType`).
    fn tile_get_item_by_id(&self, x: u16, y: u16, z: u8, item_type: u16) -> Option<ScriptItemId> {
        let _ = (x, y, z, item_type);
        None
    }

    /// `tile:getItemByGroup(ITEM_GROUP_*)` — splash by OTB group; magicfield by type tag.
    fn tile_get_item_by_group(&self, x: u16, y: u16, z: u8, group: i32) -> Option<ScriptItemId> {
        let _ = (x, y, z, group);
        None
    }

    /// `tile:queryAdd(creature[, flags])` — returns `ReturnValue` ordinal.
    fn tile_query_add_creature(
        &self,
        x: u16,
        y: u16,
        z: u8,
        creature_id: ScriptCreatureId,
        flags: u32,
    ) -> i32 {
        let _ = (x, y, z, creature_id, flags);
        1 // RETURNVALUE_NOTPOSSIBLE
    }

    /// `tile:getItemByType(ITEM_TYPE_*)` — first matching item by type_tag.
    fn tile_get_item_by_type(&self, x: u16, y: u16, z: u8, type_tag: i32) -> Option<ScriptItemId> {
        let _ = (x, y, z, type_tag);
        None
    }

    /// Walkable for rope/levitate helpers — ground present, not block-solid.
    fn tile_is_walkable(&self, x: u16, y: u16, z: u8) -> bool {
        let _ = (x, y, z);
        false
    }

    /// `MonsterType:isSummonable()` — `<flag summonable=…>`.
    fn get_monster_type_is_summonable(&self, name: &str) -> bool {
        let _ = name;
        false
    }

    /// `MonsterType:isConvinceable()` — `<flag convinceable=…>`.
    fn get_monster_type_is_convinceable(&self, name: &str) -> bool {
        let _ = name;
        false
    }

    /// `MonsterType:getManaCost()` — XML `manacost=`.
    fn get_monster_type_mana_cost(&self, name: &str) -> u32 {
        let _ = name;
        0
    }

    /// `creature:getSummons()` — creature ids with `master == self`.
    fn get_creature_summons(&self, creature_id: ScriptCreatureId) -> Vec<ScriptCreatureId> {
        let _ = creature_id;
        Vec::new()
    }

    /// `creature:getMaster()` — summoner id, or `None` if wild / no live master.
    /// TFS `luaCreatureGetMaster` (`luascript.cpp`).
    fn get_creature_master(&self, creature_id: ScriptCreatureId) -> Option<ScriptCreatureId> {
        let _ = creature_id;
        None
    }

    /// `creature:isMonster()`.
    fn is_creature_monster(&self, creature_id: ScriptCreatureId) -> bool {
        let _ = creature_id;
        false
    }

    /// Monster type name for `creature:getType()` → `MonsterType(name)`.
    fn get_creature_monster_type_name(&self, creature_id: ScriptCreatureId) -> Option<String> {
        let _ = creature_id;
        None
    }

    /// `ItemType:isCorpse()` — `corpsetype` set in items.xml.
    fn get_item_type_is_corpse(&self, item_type: u16) -> bool {
        let _ = item_type;
        false
    }

    /// `ItemType:isMovable()` — `ItemType::moveable`.
    fn get_item_type_is_movable(&self, item_type: u16) -> bool {
        let _ = item_type;
        true
    }

    /// `ItemType:isGroundTile()` — `ItemType::isGroundTile`.
    fn get_item_type_is_ground_tile(&self, item_type: u16) -> bool {
        let _ = item_type;
        false
    }

    /// NPC-7: `npc:getParameter(key)` — definition parameter map.
    fn get_npc_parameter(&self, creature_id: ScriptCreatureId, key: &str) -> Option<String> {
        let _ = (creature_id, key);
        None
    }

    /// NPC-7: `npc:isInTalkRange(player)` — same-floor focus-range check.
    fn npc_is_in_talk_range(&self, npc_id: ScriptCreatureId, player_id: ScriptCreatureId) -> bool {
        let _ = (npc_id, player_id);
        false
    }

    /// NPC-7: `npc:getFocus()` — current interlocutor creature id.
    fn get_npc_focus(&self, npc_id: ScriptCreatureId) -> Option<ScriptCreatureId> {
        let _ = npc_id;
        None
    }

    /// NPC-7: `player:getBankBalance()` — `PlayerEconomy.balance`.
    fn get_player_bank_balance(&self, creature_id: ScriptCreatureId) -> Option<u64> {
        let _ = creature_id;
        None
    }

    /// `config.lua` boolean (`freePremium`, …) — `ConfigManager::getBoolean`.
    fn get_config_bool(&self, key: &str) -> Option<bool> {
        let _ = key;
        None
    }

    /// `player:getPremiumEndsAt()` — `accounts.premium_ends_at` unix seconds.
    fn get_player_premium_ends_at(&self, creature_id: ScriptCreatureId) -> Option<u32> {
        let _ = creature_id;
        None
    }

    /// C++ `Player::isPremium` — freePremium / always-premium flag / ends_at.
    fn player_is_premium(&self, creature_id: ScriptCreatureId) -> bool {
        let _ = creature_id;
        false
    }

    /// `getWorldTime()` — TFS `LuaScriptInterface::luaGetWorldTime` (`luascript.cpp:3115`).
    /// Returns world time in game-minutes (0..1439).
    fn get_world_time(&self) -> i32 {
        0
    }

    /// `getWorldLight()` — TFS `LuaScriptInterface::luaGetWorldLight` (`luascript.cpp:3123`).
    /// Returns `(level, color)` for the current ambient light.
    fn get_world_light(&self) -> (u8, u8) {
        (0xFF, 0xD7)
    }

    /// `player:hasLearnedSpell(name)` — TFS `luaPlayerHasLearnedSpell` /
    /// `Player::hasLearnedInstantSpell`. 772 `TPlayer::SpellKnown` (`crplayer.cc:1130`).
    fn player_has_learned_spell(&self, creature_id: ScriptCreatureId, name: &str) -> bool {
        let _ = (creature_id, name);
        false
    }

    /// All instant spell defs (incl. rune-conjure instants). **Not** TFS
    /// `player:getInstantSpells` = canCast / SpellKnown filter.
    fn list_instant_spells(&self) -> Vec<ScriptInstantSpell> {
        Vec::new()
    }

    /// `player:getInstantSpells()` — TFS `luaPlayerGetInstantSpells` learn/vocation
    /// arm (no IGNORE_SPELL_CHECK). 772 GetSpellbook `SpellKnown` on TFS domain.
    fn list_player_instant_spells(&self, creature_id: ScriptCreatureId) -> Vec<ScriptInstantSpell> {
        let _ = creature_id;
        Vec::new()
    }

    /// `tile:getHouse()` — TFS `luaTileGetHouse`. `Some(house_id)` when the tile
    /// is a house (`house_id != 0`); `None` otherwise. Never `Some(0)` — Lua `0`
    /// is truthy. 772 `IsHouse` (`map.cc:2474`) is `GetHouseID != 0`.
    fn tile_get_house_id(&self, x: u16, y: u16, z: u8) -> Option<u32> {
        let _ = (x, y, z);
        None
    }
}

/// Town snapshot for Lua `Town` userdata (`luascript.cpp` `luaTownCreate`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptTownData {
    pub id: u32,
    pub name: String,
    pub temple: Position,
}

/// Instant spell snapshot for Lua `Game.getInstantSpells()` (E6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptInstantSpell {
    pub name: String,
    pub words: String,
    pub level: u32,
    pub magic_level: u32,
    pub mana: u32,
    pub mana_percent: u32,
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
