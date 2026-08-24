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

    /// SlotMap id for Lua after a successful create. Hydrates containers
    /// (TFS `Item::CreateItem` returns `Container` for container types).
    fn lua_created_item_script_id(&mut self, iid: crate::ids::ItemId) -> u64 {
        self.hydrate_container_if_needed(iid);
        iid.data().as_ffi()
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
            Ok(landed) => Ok(Some(self.lua_created_item_script_id(landed))),
            Err(_) => {
                self.items.remove(iid);
                Ok(None)
            }
        }
    }

    /// `Game.createItem(itemId[, count[, position]])` — `luaGameCreateItem`.
    ///
    /// With a position: `internalAddItem` + `FLAG_NOLIMIT`. Without: detached SlotMap item.
    /// Container types are hydrated (TFS `Item::CreateItem` → `Container`).
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
            return Ok(Some(self.lua_created_item_script_id(iid)));
        };
        let pos = Position { x, y, z };
        if self.map.get_tile(pos).is_none() {
            self.items.remove(iid);
            return Ok(None);
        }
        match self.internal_add_item_to_tile(pos, iid, CylinderFlags::NO_LIMIT) {
            Ok(landed) => Ok(Some(self.lua_created_item_script_id(landed))),
            Err(_) => {
                self.items.remove(iid);
                Ok(None)
            }
        }
    }

    /// `Game.createTile(position[, isDynamic])` — `luaGameCreateTile`.
    /// Get-or-create. `is_dynamic` is accepted (TFS DynamicTile vs StaticTile) but unused.
    pub fn lua_script_game_create_tile(
        &mut self,
        x: u16,
        y: u16,
        z: u8,
        _is_dynamic: bool,
    ) -> Result<(), String> {
        let pos = Position { x, y, z };
        if self.map.get_tile(pos).is_none() {
            self.map.insert_tile(pos, crate::tile::Tile::empty_normal());
        }
        Ok(())
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

    /// `player:setGhostMode(enabled)` — TFS `luaPlayerSetGhostMode`.
    /// Flip `ghost_mode` then appear/disappear to spectators (`can_see_creature`).
    pub fn lua_script_set_ghost_mode(
        &mut self,
        creature_u64: u64,
        enabled: bool,
    ) -> Result<(), String> {
        let Some(cid) = self.resolve_creature_u64(creature_u64) else {
            return Ok(());
        };
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return Ok(());
        };
        if p.ghost_mode == enabled {
            return Ok(());
        }
        let pos = p.base.position;
        let spectators: Vec<(tfs_rust_common::ConnId, crate::ids::CreatureId)> = self
            .spectator_conns_via_grid(pos)
            .into_iter()
            .filter_map(|conn| {
                let viewer = *self.conn_to_creature.get(&conn)?;
                (viewer != cid).then_some((conn, viewer))
            })
            .collect();

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.ghost_mode = enabled;
        }

        // TFS `Player::sendCreatureChangeVisible`: lookType 0 is the 772 sparkle.
        // Send to self and any spectator who can still see (other ghosts); others get
        // tile remove / appear.
        let outfit_bytes = {
            let Some(kind) = self.creatures.get(cid) else {
                return Ok(());
            };
            let wire_id = crate::login_out::creature_wire_id(cid, kind);
            let outfit = if enabled {
                tfs_rust_net::creature_encode::OutfitWire::default()
            } else {
                let o = &kind.base().outfit;
                tfs_rust_net::creature_encode::OutfitWire {
                    look_type: o.look_type.max(0) as u16,
                    look_head: o.look_head.clamp(0, 255) as u8,
                    look_body: o.look_body.clamp(0, 255) as u8,
                    look_legs: o.look_legs.clamp(0, 255) as u8,
                    look_feet: o.look_feet.clamp(0, 255) as u8,
                    look_addons: o.look_addons.clamp(0, 255) as u8,
                    look_mount: 0,
                    look_type_ex: 0,
                }
            };
            self.codec
                .encode_creature_outfit(wire_id, &outfit)
                .into_bytes()
        };
        if let Some(own) = self.creature_to_conn.get(&cid).copied() {
            self.enqueue_outgoing(own, outfit_bytes.clone());
        }

        for (conn, viewer) in spectators {
            if self.can_see_creature(viewer, cid) {
                self.enqueue_outgoing(conn, outfit_bytes.clone());
            } else if enabled {
                let stack_raw = self
                    .map
                    .get_tile(pos)
                    .map(|t| crate::tile::client_creature_stack_pos(t.body(), cid))
                    .unwrap_or(-1);
                self.send_creature_remove_to_conn(conn, cid, pos, stack_raw);
            } else {
                self.send_creature_appear_to_conn(conn, viewer, cid, pos);
            }
        }
        Ok(())
    }

    /// `creature:remove()` — TFS `luaCreatureRemove`. Players: forced logout.
    pub fn lua_script_creature_remove(&mut self, creature_u64: u64) -> Result<(), String> {
        let Some(cid) = self.resolve_creature_u64(creature_u64) else {
            return Ok(());
        };
        if matches!(self.creatures.get(cid), Some(CreatureKind::Player(_)))
            && let Some(conn) = self.creature_to_conn.get(&cid).copied()
        {
            self.player_logout(conn, cid, true, true);
            return Ok(());
        }
        self.remove_creature(cid);
        Ok(())
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

    fn register_ground_type(world: &mut GameWorld, id: u16) {
        use tfs_rust_content::otb::ItemType;
        let mut it = pickup_item_type(id);
        it.id = id;
        it.server_id = id;
        it.group = ItemType::GROUP_GROUND;
        let mut db = (*world.items_db).clone();
        db.items.insert(id, it);
        world.items_db = Arc::new(db);
    }

    /// Splash 2016/2019: OTB `GROUP_SPLASH` + FLAG_ALWAYSONTOP, typical order 2.
    fn register_splash_type(world: &mut GameWorld, id: u16) {
        use tfs_rust_content::otb::ItemType;
        let mut it = pickup_item_type(id);
        it.id = id;
        it.server_id = id;
        it.group = ItemType::GROUP_SPLASH;
        it.flags = 1 << 13; // FLAG_ALWAYSONTOP
        it.always_on_top_order = 2;
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

    /// R2: detached `Game.createItem(bag)` hydrates the registry so `addItem` works.
    #[test]
    fn lua_script_game_create_item_hydrates_container_for_add_item() {
        let mut world = minimal_world();
        let detached = world
            .lua_script_game_create_item(1987, 1, None)
            .unwrap()
            .expect("bag");
        let det = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(detached));
        assert!(
            world.container_registry.get(det).is_some(),
            "CreateItem hydrates Container (TFS Item::CreateItem)"
        );
        let gold = world
            .lua_script_container_add_item(detached, 2148, 1, -1, 0)
            .unwrap();
        assert!(gold.is_some(), "hydrated bag accepts addItem");
    }

    /// R3: detached `createItem` gold lands in a worn bag via `addItemEx`.
    #[test]
    fn r3_add_item_ex_moves_detached_item_into_player_backpack() {
        use crate::container::Container;
        use crate::item::Item;
        use tfs_rust_lua::LuaMoveDestination;

        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let cid = insert_player(&mut world, test_player("Quest", pos));
        let bp = world.items.insert(Item::new_single(1987));
        {
            let CreatureKind::Player(p) = world.creatures.get_mut(cid).expect("player") else {
                panic!("not player");
            };
            p.equipment_slots[2] = Some(bp);
        }
        if let Some(item) = world.items.get_mut(bp) {
            item.parent = Some(crate::cylinder::Cylinder::Inventory {
                player_id: cid,
                slot: 3,
            });
        }
        world.container_registry.register(Container::new(bp, 20));

        let gold = world
            .lua_script_game_create_item(2148, 40, None)
            .unwrap()
            .expect("gold");
        let rv = world
            .lua_script_add_item_ex(
                gold,
                LuaMoveDestination::Player {
                    creature_id: cid.data().as_ffi(),
                },
                false,
                -1,
                0,
            )
            .unwrap();
        assert_eq!(rv, ReturnValue::NoError as i32);
        let gold_id = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(gold));
        assert!(
            world.items.get(gold_id).unwrap().parent.is_some(),
            "addItemEx must stamp parent"
        );
        let again = world
            .lua_script_add_item_ex(
                gold,
                LuaMoveDestination::Player {
                    creature_id: cid.data().as_ffi(),
                },
                false,
                -1,
                0,
            )
            .unwrap();
        assert_eq!(again, ReturnValue::NotPossible as i32);
    }

    /// R3: `container:addItemEx` into a detached bag (loot `createLootItem`).
    #[test]
    fn r3_add_item_ex_into_detached_container() {
        use tfs_rust_lua::LuaMoveDestination;

        let mut world = minimal_world();
        let bag = world
            .lua_script_game_create_item(1987, 1, None)
            .unwrap()
            .expect("bag");
        let gold = world
            .lua_script_game_create_item(2148, 1, None)
            .unwrap()
            .expect("gold");
        let rv = world
            .lua_script_add_item_ex(
                gold,
                LuaMoveDestination::Container { item_id: bag },
                false,
                -1,
                0,
            )
            .unwrap();
        assert_eq!(rv, ReturnValue::NoError as i32);
        let bag_id = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(bag));
        assert_eq!(
            world.container_registry.get(bag_id).map(|c| c.size()),
            Some(1)
        );
    }

    /// R5: `Game.createTile` inserts a missing tile so `createItem` can land.
    /// Mintwallin lever: `createTile` then `createItem(1284)` — 1284 is OTB group ground.
    #[test]
    fn r5_create_tile_get_or_create_then_create_item_lands() {
        let mut world = minimal_world();
        register_ground_type(&mut world, 1284);
        let pos = Position::new(32426, 32201, 14);
        assert!(world.map.get_tile(pos).is_none());
        world
            .lua_script_game_create_tile(pos.x, pos.y, pos.z, true)
            .unwrap();
        assert!(world.map.get_tile(pos).is_some());
        world
            .lua_script_game_create_tile(pos.x, pos.y, pos.z, true)
            .unwrap();
        let item = world
            .lua_script_game_create_item(1284, 1, Some((pos.x, pos.y, pos.z)))
            .unwrap();
        assert!(item.is_some(), "createItem after createTile must land");
        let iid = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(item.unwrap()));
        let body = world.map.get_tile(pos).unwrap().body();
        assert_eq!(body.ground, Some(1284));
        assert_eq!(body.ground_item, Some(iid));
        assert!(
            body.down_items.is_empty() && body.top_items.is_empty(),
            "drawbridge must be the bank, not a stack overlay"
        );
    }

    /// TFS `addThing` replaces an existing ground item (`tile.cpp` ~852–867).
    #[test]
    fn create_item_group_ground_replaces_existing_bank() {
        let mut world = minimal_world();
        register_ground_type(&mut world, 1284);
        register_ground_type(&mut world, 493);
        let pos = Position::new(50, 50, 7);
        world
            .lua_script_game_create_tile(pos.x, pos.y, pos.z, true)
            .unwrap();
        let first = world
            .lua_script_game_create_item(1284, 1, Some((pos.x, pos.y, pos.z)))
            .unwrap()
            .expect("drawbridge");
        let first_id = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(first));
        let water = world
            .lua_script_game_create_item(493, 1, Some((pos.x, pos.y, pos.z)))
            .unwrap()
            .expect("water");
        let water_id = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(water));
        let body = world.map.get_tile(pos).unwrap().body();
        assert_eq!(body.ground, Some(493));
        assert_eq!(body.ground_item, Some(water_id));
        assert!(world.items.get(first_id).is_none(), "old ground released");
        assert!(world.items.get(water_id).is_some());
    }

    /// Rat-bridge dirt 4799 is OTB group NONE + always-on-top — must not replace water.
    #[test]
    fn create_item_always_on_top_does_not_replace_ground() {
        let mut world = minimal_world();
        register_ground_type(&mut world, 493);
        let mut dirt = pickup_item_type(4799);
        dirt.id = 4799;
        dirt.server_id = 4799;
        dirt.flags = 1 << 13; // FLAG_ALWAYSONTOP
        dirt.always_on_top_order = 1;
        let mut db = (*world.items_db).clone();
        db.items.insert(4799, dirt);
        world.items_db = Arc::new(db);

        let pos = Position::new(50, 50, 7);
        world
            .lua_script_game_create_tile(pos.x, pos.y, pos.z, true)
            .unwrap();
        let water = world
            .lua_script_game_create_item(493, 1, Some((pos.x, pos.y, pos.z)))
            .unwrap()
            .expect("water");
        let dirt_id = world
            .lua_script_game_create_item(4799, 1, Some((pos.x, pos.y, pos.z)))
            .unwrap()
            .expect("dirt");
        let water_iid = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(water));
        let dirt_iid = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(dirt_id));
        let body = world.map.get_tile(pos).unwrap().body();
        assert_eq!(body.ground, Some(493));
        assert_eq!(body.ground_item, Some(water_iid));
        assert_eq!(body.top_items.as_slice(), &[dirt_iid]);
        assert!(
            world.items.get(water_iid).is_some(),
            "water bank must survive dirt overlay"
        );
    }

    /// TFS `Tile::addThing` splash replace (`tile.cpp` ~868–882): one splash on the top stack.
    #[test]
    fn create_item_splash_replaces_existing_top_splash() {
        let mut world = minimal_world();
        register_splash_type(&mut world, 2016);
        let pos = Position::new(50, 50, 7);
        world
            .lua_script_game_create_tile(pos.x, pos.y, pos.z, true)
            .unwrap();
        let first = world
            .lua_script_game_create_item(2016, 1, Some((pos.x, pos.y, pos.z)))
            .unwrap()
            .expect("first splash");
        let first_id = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(first));
        let second = world
            .lua_script_game_create_item(2016, 5, Some((pos.x, pos.y, pos.z)))
            .unwrap()
            .expect("second splash");
        let second_id = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(second));
        let body = world.map.get_tile(pos).unwrap().body();
        assert_eq!(body.top_items.as_slice(), &[second_id]);
        assert!(world.items.get(first_id).is_none(), "old splash released");
        assert_eq!(world.items.get(second_id).map(|i| i.count), Some(5));
    }

    /// 772 `CreatePool` skips TOP (ladders) and still `Create`s the pool.
    /// Sorted insert: equal `alwaysOnTopOrder` → splash before ladder (`tile.rs` `add_top_item_at`).
    #[test]
    fn create_item_splash_lands_on_ladder_tile() {
        let mut world = minimal_world();
        register_splash_type(&mut world, 2016);
        let mut ladder = pickup_item_type(1386);
        ladder.id = 1386;
        ladder.server_id = 1386;
        ladder.flags = 1 << 13; // FLAG_ALWAYSONTOP
        ladder.always_on_top_order = 2;
        let mut db = (*world.items_db).clone();
        db.items.insert(1386, ladder);
        world.items_db = Arc::new(db);

        let pos = Position::new(50, 50, 7);
        world
            .lua_script_game_create_tile(pos.x, pos.y, pos.z, true)
            .unwrap();
        let ladder_id = world
            .lua_script_game_create_item(1386, 1, Some((pos.x, pos.y, pos.z)))
            .unwrap()
            .expect("ladder");
        let ladder_iid = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(ladder_id));
        let splash = world
            .lua_script_game_create_item(2016, 1, Some((pos.x, pos.y, pos.z)))
            .unwrap()
            .expect("772 CreatePool places splash on ladder tiles");
        let splash_iid = crate::ids::ItemId::from(slotmap::KeyData::from_ffi(splash));
        let body = world.map.get_tile(pos).unwrap().body();
        assert_eq!(
            body.top_items.as_slice(),
            &[splash_iid, ladder_iid],
            "equal alwaysOnTopOrder: splash inserts before ladder"
        );
    }

    /// Combat `create_liquid_splash` replace goes through `internal_add_item_to_tile`.
    #[test]
    fn create_liquid_splash_leaves_one_splash() {
        let mut world = minimal_world();
        register_splash_type(&mut world, 2019);
        let pos = Position::new(50, 50, 7);
        world
            .lua_script_game_create_tile(pos.x, pos.y, pos.z, true)
            .unwrap();
        world.create_liquid_splash(pos, 2019, 2);
        let first = world.map.get_tile(pos).unwrap().body().top_items[0];
        assert_eq!(world.items.get(first).map(|i| i.count), Some(2));
        world.create_liquid_splash(pos, 2019, 4);
        let body = world.map.get_tile(pos).unwrap().body();
        assert_eq!(body.top_items.len(), 1);
        assert!(world.items.get(first).is_none(), "first splash released");
        let remaining = body.top_items[0];
        assert_eq!(world.items.get(remaining).map(|i| i.count), Some(4));
        assert_eq!(world.items.get(remaining).map(|i| i.fluid_type()), Some(4));
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

    #[test]
    fn set_ghost_mode_is_readable() {
        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let cid = insert_player(&mut world, test_player("Ghost", pos));
        let id = cid.data().as_ffi();
        assert!(!world.is_creature_in_ghost_mode(id));
        world.lua_script_set_ghost_mode(id, true).unwrap();
        assert!(world.is_creature_in_ghost_mode(id));
        world.lua_script_set_ghost_mode(id, false).unwrap();
        assert!(!world.is_creature_in_ghost_mode(id));
    }

    #[test]
    fn set_ghost_mode_sends_empty_outfit_to_self() {
        use tfs_rust_common::ConnId;
        use tfs_rust_common::protocol_opcodes::server;

        let mut world = minimal_world();
        let pos = Position::new(50, 50, 7);
        ensure_walkable_tile(&mut world.map, pos, 100);
        let cid = insert_player(&mut world, test_player("Sparkle", pos));
        let guid = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.guid,
            _ => panic!("player"),
        };
        let conn = ConnId(1);
        world.register_conn_mapping(conn, cid);
        world
            .lua_script_set_ghost_mode(cid.data().as_ffi(), true)
            .unwrap();
        let pkts = world.pending_outgoing.get(&conn).expect("outfit packet");
        assert!(
            pkts.iter().any(|p| {
                p.first() == Some(&server::CREATURE_OUTFIT)
                    && p.get(1..5) == Some(&guid.to_le_bytes())
                    && p.get(5..7) == Some(&[0, 0])
            }),
            "ghost should send 0x8E lookType 0 (invisible animation) to self: {pkts:?}"
        );
    }

    #[test]
    fn creature_remove_unknown_id_is_ok() {
        let mut world = minimal_world();
        world.lua_script_creature_remove(0).unwrap();
    }

    #[test]
    fn session_ipv4_packs_low_octet_first() {
        let packed = u32::from_le_bytes([127, 0, 0, 1]);
        // Game.convertIpToString: band(ip, 0xFF) is first octet.
        assert_eq!(packed & 0xFF, 127);
        assert_eq!((packed >> 8) & 0xFF, 0);
        assert_eq!((packed >> 24) & 0xFF, 1);
    }
}
