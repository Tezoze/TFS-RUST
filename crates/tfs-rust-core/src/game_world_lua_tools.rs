//! Lua Gap 3 mutation appliers — tools-script verbs (`addSkillTries`, tile/game create, target combat).
//!
//! Domain: TFS `luascript.cpp` (`luaPlayerAddSkillTries`, `luaTileAddItem`,
//! `luaGameCreateItem`, `luaDoTargetCombat`).
//! Outcomes: 772 skill tries via `skill_increase`; PZ-lock is a read (`earliest_protection_zone_round`).

use slotmap::Key;
use tfs_rust_common::Position;
use tfs_rust_common::enums::CombatType;

use crate::combat::aoe::combat_type_from_lua;
use crate::combat::{CombatDamage, CombatParams};
use crate::creature::CreatureKind;
use crate::cylinder::CylinderFlags;
use crate::game_world::GameWorld;
use crate::item::Item;
use crate::player::combat::SkillNr;
use crate::return_value::ReturnValue;

impl GameWorld {
    /// `player:addSkillTries(skill, tries)` — `luaPlayerAddSkillTries` → `addSkillAdvance`.
    ///
    /// Does **not** multiply by `rateSkill`. The data-pack wrapper
    /// (`data/lib/core/player.lua`) sets `APPLY_SKILL_MULTIPLIER = false` so TFS
    /// `Player:onGainSkillTries` returns the raw try count.
    pub fn lua_script_add_skill_tries(
        &mut self,
        creature_u64: u64,
        skill: i32,
        tries: u64,
    ) -> Result<bool, String> {
        let Some(cid) = self.resolve_creature_u64(creature_u64) else {
            return Ok(false);
        };
        let Some(skill_nr) = SkillNr::from_tfs_skill_id(skill) else {
            // C++ still returns `true` when the player userdata exists.
            return Ok(matches!(
                self.creatures.get(cid),
                Some(CreatureKind::Player(_))
            ));
        };
        let profile = self.mechanics.profile;
        let levels_gained = {
            let hooks = &self.mechanics.hooks;
            let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
                return Ok(false);
            };
            p.skill_increase(skill_nr, tries, &profile, hooks)
        };
        self.notify_skill_tries_gained(cid, skill_nr, levels_gained);
        Ok(true)
    }

    fn lua_create_item_instance(&self, item_type: u16, count: u16) -> Option<Item> {
        let it = self.items_db.items.get(&item_type)?;
        let mut count = count;
        if it.stackable() {
            count = count.clamp(1, 100);
        } else if count == 0 {
            count = 1;
        }
        Some(Item::new(item_type, count))
    }

    /// `tile:addItem(itemId[, count[, flags]])` — `luaTileAddItem` → `internalAddItem`.
    /// Returns the SlotMap id that landed on the tile (may differ after stack merge).
    pub fn lua_script_tile_add_item(
        &mut self,
        x: u16,
        y: u16,
        z: u8,
        item_type: u16,
        count: u16,
        flags: u32,
    ) -> Result<Option<u64>, String> {
        let pos = Position { x, y, z };
        if self.map.get_tile(pos).is_none() {
            return Ok(None);
        }
        let Some(item) = self.lua_create_item_instance(item_type, count) else {
            return Ok(None);
        };
        let iid = self.items.insert(item);
        let cyl_flags = CylinderFlags { bits: flags };
        match self.internal_add_item_to_tile(pos, iid, cyl_flags) {
            Ok(landed) => Ok(Some(landed.data().as_ffi())),
            Err(_) => {
                self.items.remove(iid);
                Ok(None)
            }
        }
    }

    /// `Game.createItem(itemId[, count[, position]])` — `luaGameCreateItem`.
    ///
    /// With a position: `internalAddItem` + `FLAG_NOLIMIT`. Without: detached SlotMap item.
    pub fn lua_script_game_create_item(
        &mut self,
        item_type: u16,
        count: u16,
        position: Option<(u16, u16, u8)>,
    ) -> Result<Option<u64>, String> {
        let Some(item) = self.lua_create_item_instance(item_type, count) else {
            return Ok(None);
        };
        let iid = self.items.insert(item);
        let Some((x, y, z)) = position else {
            return Ok(Some(iid.data().as_ffi()));
        };
        let pos = Position { x, y, z };
        if self.map.get_tile(pos).is_none() {
            self.items.remove(iid);
            return Ok(None);
        }
        match self.internal_add_item_to_tile(pos, iid, CylinderFlags::NO_LIMIT) {
            Ok(landed) => Ok(Some(landed.data().as_ffi())),
            Err(_) => {
                self.items.remove(iid);
                Ok(None)
            }
        }
    }

    /// `doTargetCombat` / `doTargetCombatHealth` — `luaDoTargetCombat`.
    /// Single target; `attacker = None` is environment damage (`cid == 0`).
    pub fn lua_script_target_combat_health(
        &mut self,
        attacker_u64: Option<u64>,
        target_u64: u64,
        combat_type_lua: i32,
        damage_min: i32,
        damage_max: i32,
        effect: i32,
    ) -> Result<bool, String> {
        let Some(target_id) = self.resolve_creature_u64(target_u64) else {
            return Ok(false);
        };
        let attacker_id = attacker_u64.and_then(|id| self.resolve_creature_u64(id));
        let combat_type = combat_type_from_lua(combat_type_lua);
        // TFS `Combat::canDoCombat` group flags — `doTargetCombat` is the
        // single-target back door around `Combat:execute`. Healing still applies.
        if combat_type != CombatType::Healing
            && let Some(attacker) = attacker_id
            && self.player_group_blocks_attack_on(attacker, target_id)
        {
            if let Some(conn) = self.conn_for_creature(attacker) {
                let rv = match self.creatures.get(target_id) {
                    Some(CreatureKind::Player(_)) => ReturnValue::YouMayNotAttackThisPlayer,
                    _ => ReturnValue::YouMayNotAttackThisCreature,
                };
                self.send_cancel_message(conn, rv);
            }
            return Ok(false);
        }
        let target_pos = self
            .creatures
            .get(target_id)
            .map(|k| k.position())
            .unwrap_or(Position { x: 0, y: 0, z: 0 });

        let value = crate::combat::uniform_random_glibc(&self.parity_rng, damage_min, damage_max);
        let notify_snap = self.combat_notify_snapshot(target_id);
        let hp_before = self
            .creatures
            .get(target_id)
            .map(|k| k.base().health)
            .unwrap_or(0);

        let mut physical_armor: Option<i32> = None;
        let signed_value = if combat_type == CombatType::Healing {
            value.max(0)
        } else if combat_type == CombatType::Physical {
            let (abs_dmg, armor) =
                self.mitigate_physical_spell_damage(target_id, target_pos, value, false, false);
            physical_armor = armor;
            -abs_dmg
        } else {
            -value.abs()
        };

        let damage = CombatDamage {
            primary: (combat_type, signed_value),
            secondary: (CombatType::Undefined, 0),
        };
        let params = CombatParams {
            primary_type: combat_type,
            dispel: None,
            apply_condition: None,
            armor: physical_armor,
        };
        self.combat_execute_with_stimulus(attacker_id, target_id, &damage, &params);

        if effect > 0 {
            self.broadcast_magic_effect(target_pos, effect as u8);
        }

        if let Some(snap) = notify_snap {
            if combat_type == CombatType::Healing {
                self.notify_creature_healed(target_id, snap);
            } else {
                let hp_after = self
                    .creatures
                    .get(target_id)
                    .map(|k| k.base().health)
                    .unwrap_or(0);
                let damage_done = (hp_before - hp_after).max(0);
                self.notify_player_combat_damage(
                    attacker_id,
                    target_id,
                    damage_done,
                    combat_type,
                    snap,
                );
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_harness::{
        ensure_walkable_tile, insert_player, minimal_world, pickup_item_type, test_player,
    };
    use crate::tile::{Tile, TileBody};
    use std::sync::Arc;
    use tfs_rust_common::ScriptContext;
    use tfs_rust_common::enums::ZoneType;

    fn register_item_type(world: &mut GameWorld, id: u16, stackable: bool) {
        let mut it = pickup_item_type(id);
        it.id = id;
        if stackable {
            it.flags = 1 << 7;
        }
        let mut db = (*world.items_db).clone();
        db.items.insert(id, it);
        world.items_db = Arc::new(db);
    }

    #[test]
    fn lua_script_add_skill_tries_advances_fishing() {
        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        let cid = insert_player(&mut world, test_player("Fisher", pos));
        let id = cid.data().as_ffi();
        let before = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.skills.fishing_tries,
            _ => panic!("player"),
        };
        assert!(world.lua_script_add_skill_tries(id, 6, 1).unwrap());
        let after = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.skills.fishing_tries,
            _ => panic!("player"),
        };
        assert_eq!(after, before + 1);
        assert_eq!(world.get_player_effective_skill(id, 6), Some(10));
        assert!(!world.lua_script_add_skill_tries(0, 6, 1).unwrap());
    }

    #[test]
    fn lua_script_tile_add_item_and_create_item_land_on_tile() {
        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        register_item_type(&mut world, 2667, true);

        let added = world
            .lua_script_tile_add_item(50, 50, 7, 2667, 1, 0)
            .unwrap();
        assert!(added.is_some());
        let iid = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(added.unwrap()));
        assert_eq!(world.items.get(iid).map(|i| i.item_type), Some(2667));

        let created = world
            .lua_script_game_create_item(2667, 1, Some((50, 50, 7)))
            .unwrap();
        assert!(created.is_some());

        let detached = world.lua_script_game_create_item(2667, 1, None).unwrap();
        assert!(detached.is_some());
        let det = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(detached.unwrap()));
        assert!(world.items.get(det).is_some());
        assert!(world.items.get(det).unwrap().parent.is_none());
    }

    #[test]
    fn player_is_pz_locked_matches_protection_zone_round() {
        let mut world = minimal_world();
        world.round_nr = 100;
        let pos = Position::new(50, 50, 7);
        let cid = insert_player(&mut world, test_player("Locked", pos));
        let id = cid.data().as_ffi();
        assert_eq!(world.player_is_pz_locked(id), Some(false));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            p.earliest_protection_zone_round = 160;
        }
        assert_eq!(world.player_is_pz_locked(id), Some(true));
    }

    #[test]
    fn tile_get_bottom_creature_is_oldest() {
        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let a = insert_player(&mut world, test_player("A", pos));
        let b = insert_player(&mut world, test_player("B", pos));
        if let Some(tile) = world.map.get_tile_mut(pos) {
            tile.add_creature(a);
            tile.add_creature(b);
        }
        let bottom = world.tile_get_bottom_creature(50, 50, 7).unwrap();
        assert_eq!(bottom, a.data().as_ffi());
    }

    #[test]
    fn get_fluid_type_and_ground_item() {
        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        register_item_type(&mut world, 2016, false);
        let mut splash = Item::new(2016, 1);
        splash.set_fluid_type(5);
        let iid = world.items.insert(splash);
        world.map.insert_tile(
            pos,
            Tile::Normal(TileBody {
                ground: Some(100),
                ground_item: Some(iid),
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: 0,
                zone: ZoneType::Normal,
            }),
        );
        let sid = iid.data().as_ffi();
        assert_eq!(world.get_item_data(sid).map(|d| d.fluid_type), Some(5));
        assert_eq!(world.tile_get_ground_item(50, 50, 7), Some(sid));
    }

    #[test]
    fn target_combat_with_no_caster_deals_hp() {
        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let cid = insert_player(&mut world, test_player("Victim", pos));
        let id = cid.data().as_ffi();
        let hp_before = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.base.health,
            _ => panic!("player"),
        };
        assert!(
            world
                .lua_script_target_combat_health(None, id, 1, -50, -50, 0)
                .unwrap()
        );
        let hp_after = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.base.health,
            _ => panic!("player"),
        };
        assert!(
            hp_after < hp_before,
            "expected HP loss: {hp_before} → {hp_after}"
        );
    }
}
