//! Lua `ScriptContext` bridge over live `GameWorld` state.
//!
//! - TFS script API surface — `luascript.cpp` / `game.cpp`.

use slotmap::Key;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;

impl tfs_rust_common::ScriptContext for GameWorld {
    fn get_creature(
        &self,
        id: tfs_rust_common::ScriptCreatureId,
    ) -> Option<tfs_rust_common::ScriptCreatureData> {
        self.creatures
            .iter()
            .find_map(|(cid, k)| {
                if cid.data().as_ffi() != id {
                    return None;
                }
                Some(match k {
                    CreatureKind::Player(p) => Some(tfs_rust_common::ScriptCreatureData {
                        name: p.base.name.clone(),
                        guid: p.guid,
                    }),
                    CreatureKind::Monster(m) => Some(tfs_rust_common::ScriptCreatureData {
                        name: m.base.name.clone(),
                        guid: 0, // Monsters don't have GUIDs
                    }),
                    CreatureKind::Npc(n) => Some(tfs_rust_common::ScriptCreatureData {
                        name: n.base.name.clone(),
                        guid: 0, // NPCs don't have GUIDs
                    }),
                })
            })
            .flatten()
    }

    fn get_item(
        &self,
        id: tfs_rust_common::ScriptItemId,
    ) -> Option<tfs_rust_common::ScriptItemRef> {
        self.items
            .iter()
            .find(|(item_id, _)| item_id.data().as_ffi() == id)
            .map(|_| tfs_rust_common::ScriptItemRef(id))
    }

    fn get_config_string(&self, key: &str) -> Option<String> {
        self.config.get_string(key).ok()
    }

    fn get_player_slot_item_id(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        slot: u8,
    ) -> Option<tfs_rust_common::ScriptItemId> {
        let cid = self
            .creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == creature_id)
            .map(|(k, _)| k)?;
        self.get_player_inventory_item(cid, slot)
            .map(|i| i.data().as_ffi())
    }

    fn get_player_capacity(&self, creature_id: tfs_rust_common::ScriptCreatureId) -> Option<u32> {
        let _cid = self
            .creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == creature_id)
            .map(|(k, _)| k)?;
        self.player_capacity_u32(_cid)
    }

    fn get_player_free_capacity(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
    ) -> Option<u32> {
        let _cid = self
            .creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == creature_id)
            .map(|(k, _)| k)?;
        self.player_free_capacity_u32(_cid)
    }

    fn get_player_item_type_count(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        item_id: u16,
        sub_type: i32,
    ) -> Option<u32> {
        let cid = self
            .creatures
            .iter()
            .find(|(k, _)| k.data().as_ffi() == creature_id)
            .map(|(k, _)| k)?;
        Some(self.player_get_item_type_count(cid, item_id, sub_type))
    }

    fn get_item_data(
        &self,
        id: tfs_rust_common::ScriptItemId,
    ) -> Option<tfs_rust_common::ScriptItemData> {
        let iid = self
            .items
            .iter()
            .find(|(item_id, _)| item_id.data().as_ffi() == id)
            .map(|(k, _)| k)?;
        let item = self.items.get(iid)?;
        let it = self.items_db.items.get(&item.item_type);
        let tw = it.map(|t| t.weight).unwrap_or(0);
        let stack = it.map(|t| t.stackable()).unwrap_or(false);
        let w = item.total_weight_oz(tw, stack);
        Some(tfs_rust_common::ScriptItemData {
            item_type: item.item_type,
            count: item.count,
            weight: w,
            name: it.map(|t| t.name.clone()).unwrap_or_default(),
            action_id: item.action_id(),
            unique_id: u32::from(item.unique_id()),
            is_store_item: item.is_store_item(),
        })
    }

    fn get_item_type_id_by_name(&self, name: &str) -> Option<u16> {
        self.item_type_id_by_name(name)
    }

    fn find_player_item_by_type(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        item_id: u16,
        depth_search: bool,
        sub_type: i32,
    ) -> Option<tfs_rust_common::ScriptItemId> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.find_item_of_type(cid, item_id, depth_search, sub_type)
            .map(GameWorld::item_to_script_id)
    }

    fn is_registered_container(&self, item_id: tfs_rust_common::ScriptItemId) -> bool {
        self.resolve_item_u64(item_id)
            .is_some_and(|i| self.script_is_registered_container(i))
    }

    fn get_container_data(
        &self,
        item_id: tfs_rust_common::ScriptItemId,
    ) -> Option<tfs_rust_common::ScriptContainerData> {
        let iid = self.resolve_item_u64(item_id)?;
        self.script_container_data(iid)
    }

    fn get_container_item_at(
        &self,
        container_id: tfs_rust_common::ScriptItemId,
        index: u32,
    ) -> Option<tfs_rust_common::ScriptItemId> {
        let cid = self.resolve_item_u64(container_id)?;
        self.script_container_item_at(cid, index)
            .map(GameWorld::item_to_script_id)
    }

    fn get_container_items(
        &self,
        container_id: tfs_rust_common::ScriptItemId,
    ) -> Vec<tfs_rust_common::ScriptItemId> {
        let Some(root) = self.resolve_item_u64(container_id) else {
            return Vec::new();
        };
        self.script_container_items(root)
            .into_iter()
            .map(GameWorld::item_to_script_id)
            .collect()
    }

    fn container_has_item(
        &self,
        container_id: tfs_rust_common::ScriptItemId,
        item_id: tfs_rust_common::ScriptItemId,
    ) -> bool {
        let (Some(root), Some(item)) = (
            self.resolve_item_u64(container_id),
            self.resolve_item_u64(item_id),
        ) else {
            return false;
        };
        self.script_container_has_item(root, item)
    }

    fn get_container_item_count_by_id(
        &self,
        container_id: tfs_rust_common::ScriptItemId,
        item_type: u16,
        sub_type: i32,
    ) -> u32 {
        let Some(root) = self.resolve_item_u64(container_id) else {
            return 0;
        };
        self.script_container_item_count_by_id(root, item_type, sub_type)
    }

    fn get_player_container_id(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        container_id: tfs_rust_common::ScriptItemId,
    ) -> Option<u8> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        let root = self.resolve_item_u64(container_id)?;
        self.script_player_container_id(cid, root)
    }

    fn get_player_container_by_cid(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        client_cid: u8,
    ) -> Option<tfs_rust_common::ScriptItemId> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.script_player_container_by_cid(cid, client_cid)
            .map(GameWorld::item_to_script_id)
    }

    fn get_player_container_index(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        client_cid: u8,
    ) -> Option<u16> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.script_player_container_index(cid, client_cid)
    }

    fn get_item_parent(
        &self,
        item_id: tfs_rust_common::ScriptItemId,
    ) -> Option<tfs_rust_common::ScriptCylinder> {
        let iid = self.resolve_item_u64(item_id)?;
        self.script_item_parent(iid)
    }

    fn get_item_top_parent(
        &self,
        item_id: tfs_rust_common::ScriptItemId,
    ) -> Option<tfs_rust_common::ScriptCylinder> {
        let iid = self.resolve_item_u64(item_id)?;
        self.script_item_top_parent(iid)
    }

    fn get_item_position(
        &self,
        item_id: tfs_rust_common::ScriptItemId,
    ) -> Option<tfs_rust_common::Position> {
        let iid = self.resolve_item_u64(item_id)?;
        self.script_item_position(iid)
    }

    fn get_player_food(&self, creature_id: tfs_rust_common::ScriptCreatureId) -> Option<u32> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => Some(p.food_remaining),
            _ => None,
        })
    }

    /// `player:getLevel()` — `Player::getLevel` (`player.h`).
    fn get_player_level(&self, creature_id: tfs_rust_common::ScriptCreatureId) -> Option<i32> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => Some(p.level),
            _ => None,
        })
    }

    /// `player:getAccountType()` — `accounts.type` tier (`enums.h:80-85`).
    fn get_player_account_type(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
    ) -> Option<u8> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => Some(p.account_type),
            _ => None,
        })
    }

    /// `player:getVocation():getId()` backing read — `players.vocation`.
    fn get_player_vocation_id(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
    ) -> Option<i32> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => Some(p.vocation_id),
            _ => None,
        })
    }

    /// `player:hasFlag(flag)` — resolved `groups.xml` flag bits for `group_id`.
    /// Reuses `GameWorld::player_has_flag` (`player/stats.rs`), which resolves
    /// `groups.xml` via `flags_for_group` and tests the bit.
    fn player_has_flag(&self, creature_id: tfs_rust_common::ScriptCreatureId, flag: u64) -> bool {
        let Some(cid) = self.resolve_creature_from_script(creature_id) else {
            return false;
        };
        GameWorld::player_has_flag(self, cid, flag)
    }

    /// `player:getCondition(type, id, subId)` — `luascript.cpp:2116`
    /// `Creature::getCondition`. LUA-4 read; scans active conditions for a
    /// match on `(ctype, sub_id)` and returns remaining ticks. `ctype` is the
    /// Lua-facing 772 bit-flag value, mapped via
    /// `condition_type_from_lua`.
    fn get_creature_condition(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
        ctype: i32,
        _cond_id: i32,
        sub_id: u32,
    ) -> Option<i32> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        let rust_ctype = crate::game_world_chat::condition_type_from_lua(ctype);
        self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => {
                for cond in &p.base.active_conditions {
                    if cond.ctype == rust_ctype && cond.sub_id == sub_id {
                        if let crate::condition::ConditionData::Generic { ticks } = cond.data {
                            return Some(ticks);
                        }
                    }
                }
                None
            }
            _ => None,
        })
    }

    /// `Player(name)` constructor — `luascript.cpp` `luaPlayerCreate`. LUA-4
    /// read; resolves an online player by name via `player_by_name`.
    fn get_player_by_name(&self, name: &str) -> Option<tfs_rust_common::ScriptCreatureId> {
        self.player_by_name
            .get(name)
            .map(|cid| Self::creature_to_script_id(*cid))
    }

    /// `player:getGroup():getId()` backing read — `players.group_id`.
    /// CH-6 talkaction access gating.
    fn get_player_group_id(&self, creature_id: tfs_rust_common::ScriptCreatureId) -> Option<u16> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => Some(p.group_id),
            _ => None,
        })
    }

    /// `group:getAccess()` backing read — `groups.xml` `access` flag.
    /// CH-6 talkaction access gating.
    fn get_group_access(&self, group_id: u16) -> bool {
        self.groups.groups.get(&group_id).is_some_and(|g| g.access)
    }

    /// `player:getPosition()` backing read — `Creature::getPosition`.
    /// CH-6 talkaction `sendMagicEffect` at player position.
    fn get_player_position(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
    ) -> Option<tfs_rust_common::Position> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.creatures.get(cid).map(|k| k.position())
    }

    /// `player:getDirection()` — `Creature::getDirection` (`creature.h`).
    /// PC-3a: spell variant construction offsets the center position by one
    /// tile in the player's facing direction when `needDirection(true)` is set.
    fn get_player_direction(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
    ) -> Option<u8> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.creatures.get(cid).map(|k| k.base().direction as u8)
    }

    /// `ItemType:isStackable()` backing read — `ItemType::stackable`.
    /// CH-6 talkaction `/i` count clamping.
    fn get_item_type_is_stackable(&self, item_type: u16) -> bool {
        self.items_db.stackable_for_server(item_type)
    }

    /// `ItemType:isFluidContainer()` backing read — `ItemType::isFluidContainer`.
    /// CH-6 talkaction `/i` count clamping.
    fn get_item_type_is_fluid_container(&self, item_type: u16) -> bool {
        self.items_db
            .items
            .get(&item_type)
            .is_some_and(|t| t.is_fluid_container())
    }

    /// `ItemType:getCharges()` — `ItemType::charges` (`src/items.h`).
    /// PC-3a Phase 5: `Player:conjureItem` charge fallback.
    fn get_item_type_charges(&self, item_type: u16) -> u32 {
        self.items_db
            .items
            .get(&item_type)
            .map(|t| t.charges)
            .unwrap_or(0)
    }

    /// `item:hasAttribute(key)` — `ItemAttributes::hasAttribute` (`src/item.h`).
    /// PC-3a Phase 5: `conjureItem` checks `ITEM_ATTRIBUTE_DURATION`.
    fn item_has_attribute(
        &self,
        item_id: tfs_rust_common::ScriptItemId,
        attr_bits: u32,
    ) -> bool {
        let Some(iid) = self.resolve_item_u64(item_id) else {
            return false;
        };
        let Some(item) = self.items.get(iid) else {
            return false;
        };
        let Some(attrs) = item.attributes.as_ref() else {
            return false;
        };
        (attr_bits & attrs.attribute_bits()) != 0
    }

    /// `group:hasFlag(flag)` — `Group::flags & flag` (`src/groups.cpp`).
    /// PC-3a Phase 5: `conjureItem` dual-hand infinite-mana gate.
    fn group_has_flag(&self, group_id: u16, flag: u64) -> bool {
        let bits = crate::player_flags::flags_for_group(&self.groups, group_id);
        crate::player_flags::has_player_flag(bits, flag)
    }

    /// `player:getMana()` — `Player::getMana` (`player.h`).
    /// PC-3a Phase 5: `conjureItem` dual-hand second-conjure mana check.
    fn get_player_mana(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
    ) -> Option<i32> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => Some(p.mana),
            _ => None,
        })
    }

    /// `player:getMagicLevel()` — `Player::getMagicLevel` (`player.h`).
    /// PC-3a Phase 1: value-callback spells call `self:getMagicLevel()` inside
    /// `functions.lua` (`computeDamage` / `computeHealing` / `computeSkillDamage`).
    fn get_player_magic_level(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
    ) -> Option<i32> {
        let cid = self.resolve_creature_from_script(creature_id)?;
        self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => Some(p.skills.maglevel),
            _ => None,
        })
    }

    /// Weapon combat parameters for the SKILL value callback
    /// (`CALLBACK_PARAM_SKILLVALUE`). C++ `Combat::getCombatDamage` —
    /// `combat.cpp:1155-1163`: `player->getWeaponSkill()`,
    /// `player->getWeapon()->getAttack()`, `player->getAttackFactor()`.
    fn get_player_weapon_combat_params(
        &self,
        creature_id: tfs_rust_common::ScriptCreatureId,
    ) -> tfs_rust_common::WeaponCombatParams {
        let Some(cid) = self.resolve_creature_from_script(creature_id) else {
            return tfs_rust_common::WeaponCombatParams::default();
        };
        // TFS `Player::getAttackFactor` — `player.cpp`. Defaults to 1.0;
        // modified by conditions (e.g. haste/slow affect attack frequency,
        // not the factor itself in 772). We return 1.0 unless a condition
        // override is wired (future work).
        let attack_factor = 1.0;
        let weapon = self.player_get_weapon(cid, false);
        let skill = self.player_get_weapon_skill(cid, weapon);
        let attack = weapon
            .and_then(|iid| self.items.get(iid))
            .and_then(|item| {
                // C++ `Item::getAttack` — attribute overrides `ItemType::attack`.
                item.attributes
                    .as_ref()
                    .and_then(|a| a.get_attack())
                    .or_else(|| self.items_db.items.get(&item.item_type).map(|t| t.attack))
            })
            .unwrap_or(0);
        tfs_rust_common::WeaponCombatParams {
            skill,
            attack,
            attack_factor,
        }
    }

    /// Creatures on area offsets — PC-3a Phase 3 `combat:getTargets`.
    fn get_creatures_on_area(
        &self,
        center_x: u16,
        center_y: u16,
        center_z: u8,
        offsets: &[(i32, i32)],
    ) -> Vec<tfs_rust_common::ScriptCreatureId> {
        let mut out = Vec::new();
        for &(dx, dy) in offsets {
            let tx = center_x as i32 + dx;
            let ty = center_y as i32 + dy;
            if tx < 0 || ty < 0 {
                continue;
            }
            let pos = tfs_rust_common::Position {
                x: tx as u16,
                y: ty as u16,
                z: center_z,
            };
            if let Some(tile) = self.map.get_tile(pos) {
                for &cid in &tile.body().creatures {
                    out.push(Self::creature_to_script_id(cid));
                }
            }
        }
        out
    }

    fn is_creature_player(&self, creature_id: tfs_rust_common::ScriptCreatureId) -> bool {
        let Some(cid) = self.resolve_creature_from_script(creature_id) else {
            return false;
        };
        matches!(self.creatures.get(cid), Some(CreatureKind::Player(_)))
    }

    fn tile_exists(&self, x: u16, y: u16, z: u8) -> bool {
        self.map
            .get_tile(tfs_rust_common::Position { x, y, z })
            .is_some()
    }

    fn tile_has_property(&self, x: u16, y: u16, z: u8, prop: i32) -> bool {
        // Only CONST_PROP_BLOCKSOLID (0) is needed for field rune gates.
        // C++ `Tile::hasProperty` — `tile.cpp:27` / `Item::hasProperty`.
        if prop != 0 {
            return false;
        }
        let pos = tfs_rust_common::Position { x, y, z };
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        let body = tile.body();
        if let Some(ground_type) = body.ground {
            if self
                .items_db
                .items
                .get(&ground_type)
                .is_some_and(|t| t.block_solid())
            {
                return true;
            }
        }
        for &iid in body.down_items.iter().chain(body.top_items.iter()) {
            if let Some(item) = self.items.get(iid) {
                if self
                    .items_db
                    .items
                    .get(&item.item_type)
                    .is_some_and(|t| t.block_solid())
                {
                    return true;
                }
            }
        }
        false
    }

    fn get_world_type(&self) -> i32 {
        match self.pvp_config.world_type {
            tfs_rust_common::WorldType::NoPvp => 0,
            tfs_rust_common::WorldType::Pvp => 1,
            tfs_rust_common::WorldType::PvpEnforced => 2,
        }
    }

    fn get_monster_type_look_type(&self, name: &str) -> Option<i32> {
        self.monsters_db
            .get_by_name(name)
            .map(|m| m.outfit.look_type)
    }

    fn get_monster_type_is_illusionable(&self, name: &str) -> bool {
        self.monsters_db
            .get_by_name(name)
            .map(|m| m.flags.illusionable)
            .unwrap_or(false)
    }

    fn monster_type_exists(&self, name: &str) -> bool {
        self.monsters_db.get_by_name(name).is_some()
    }
}
