//! Magic-field place / step-in damage — TFS `MagicField` domain, 772 DoT outcomes.
//!
//! Domain: TFS `MagicField::onStepInField` / `MoveEvent::AddItemField` — `combat.cpp`,
//! `movement.cpp`.
//! Outcomes: 772 `DAMAGE_*_PERIODIC` → burning/poison/energy timers (`crmain.cc:582-612`,
//! `crskill.cc` TSkillBurning/Poison/Energy); field kind from items.xml `field` attr.

use tfs_rust_common::enums::CombatType;
use tfs_rust_common::Position;
use tfs_rust_content::items::FieldDamageType;

use crate::combat::{CombatDamage, CombatParams};
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};

impl GameWorld {
    /// TFS `MoveEvent::AddItemField` — apply a newly placed magic field to every creature
    /// already on the tile (`movement.cpp:672-680`).
    pub(crate) fn apply_magic_field_to_tile_creatures(
        &mut self,
        pos: Position,
        field_item_id: ItemId,
    ) {
        let Some(server_id) = self.items.get(field_item_id).map(|i| i.item_type) else {
            return;
        };
        if !self
            .items_db
            .items
            .get(&server_id)
            .is_some_and(|t| t.is_magic_field())
        {
            return;
        }
        let targets: Vec<CreatureId> = self
            .map
            .get_tile(pos)
            .map(|t| t.body().creatures.clone())
            .unwrap_or_default();
        for cid in targets {
            self.apply_magic_field_to_creature(cid, server_id);
        }
    }

    /// Apply every magic field on `pos` to `cid` — TFS `StepInField` after a walk lands.
    pub(crate) fn apply_magic_fields_under_creature(&mut self, cid: CreatureId, pos: Position) {
        let field_types: Vec<u16> = {
            let Some(tile) = self.map.get_tile(pos) else {
                return;
            };
            let body = tile.body();
            body.down_items
                .iter()
                .chain(body.top_items.iter())
                .filter_map(|&iid| {
                    let sid = self.items.get(iid)?.item_type;
                    self.items_db
                        .items
                        .get(&sid)
                        .filter(|t| t.is_magic_field())
                        .map(|_| sid)
                })
                .collect()
        };
        for sid in field_types {
            self.apply_magic_field_to_creature(cid, sid);
        }
    }

    /// TFS `MagicField::onStepInField` — `combat.cpp:1443`.
    ///
    /// Instant `initdamage` (items.xml) then 772 `DAMAGE_*_PERIODIC` arm (`crmain.cc:582-613`).
    /// `field.cycles` is the 772 `Damage` argument (fire `/10`, energy `/20`, poison pool).
    pub(crate) fn apply_magic_field_to_creature(
        &mut self,
        target: CreatureId,
        field_server_id: u16,
    ) {
        let Some(field_kind) = self.items_db.avoid_damage_type(field_server_id) else {
            return;
        };
        // Meaning-harmless Trap Damage: `!IsPeaceful` else Effect poff (`moveuse.dat`;
        // `crmain.cc:900`, `crnonpl.cc:2295`). `field.skippeaceful` — not hardcoded IDs.
        // Check before race immunity so players/NPCs/summons get poff, not a silent skip.
        let skip_peaceful = self
            .items_db
            .items
            .get(&field_server_id)
            .and_then(|t| t.xml_attributes.get("field.skippeaceful"))
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        if skip_peaceful && self.creature_is_peaceful(target) {
            if let Some(pos) = self.creatures.get(target).map(|c| c.base().position) {
                self.broadcast_magic_effect(pos, 3); // CONST_ME_POFF
            }
            return;
        }
        if self.creature_immune_to_field(target, field_kind) {
            return;
        }

        let attrs = self
            .items_db
            .items
            .get(&field_server_id)
            .map(|t| &t.xml_attributes);
        let init_damage = attrs
            .and_then(|a| a.get("field.initdamage"))
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0)
            .abs();
        let cycles = attrs
            .and_then(|a| a.get("field.cycles"))
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0)
            .abs();

        let instant_combat = match field_kind {
            FieldDamageType::Fire => CombatType::Fire,
            FieldDamageType::Energy => CombatType::Energy,
            FieldDamageType::Poison => CombatType::Earth,
        };

        if init_damage > 0 {
            let snap = self.combat_notify_snapshot(target);
            let damage = CombatDamage {
                primary: (instant_combat, -(init_damage)),
                secondary: (CombatType::Undefined, 0),
            };
            let damage_done =
                self.combat_execute_with_stimulus(None, target, &damage, &CombatParams::default());
            // M2 — `damage_done` is the real `Damage` scalar (includes mana-shield absorb).
            if let Some(snap) = snap {
                self.notify_player_combat_damage(None, target, damage_done, instant_combat, snap);
            }
        }

        // Target may have died from init damage.
        if self.creatures.get(target).is_none() {
            return;
        }

        // items.xml `field.cycles` is the 772 `Damage` argument to `DAMAGE_*_PERIODIC`
        // (`crmain.cc:600,610`): fire `SetTimer(SKILL_BURNING, Damage/10)`, energy
        // `SetTimer(SKILL_ENERGY, Damage/20)`, poison `SetTimer(SKILL_POISON, Damage)`.
        // Stock pack: fire 70 → Cycle 7, energy 25 → Cycle 1, poison 100 → pool 100.
        // Do **not** multiply fire/energy by 10/20 — that treated cycles as Event count
        // and made energy tick 25 times (~4 min) instead of once.
        let (periodic, strength) = match field_kind {
            FieldDamageType::Fire => {
                let interval = self.mechanics.profile.conditions.fire.ticks.max(1);
                let damage = if cycles > 0 {
                    cycles
                } else {
                    interval.saturating_mul(10).max(10)
                };
                (CombatType::FirePeriodic, damage)
            }
            FieldDamageType::Energy => {
                let interval = self.mechanics.profile.conditions.energy.ticks.max(1);
                let damage = if cycles > 0 {
                    cycles
                } else {
                    interval.saturating_mul(20).max(20)
                };
                (CombatType::EnergyPeriodic, damage)
            }
            FieldDamageType::Poison => {
                let rank = if cycles > 0 { cycles } else { 1 };
                (CombatType::PoisonPeriodic, rank.max(1))
            }
        };
        let _ = self.combat_execute_with_stimulus(
            None,
            target,
            &CombatDamage {
                primary: (periodic, -strength),
                secondary: (CombatType::Undefined, 0),
            },
            &CombatParams::default(),
        );
    }

    fn creature_immune_to_field(&self, cid: CreatureId, kind: FieldDamageType) -> bool {
        match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => match kind {
                FieldDamageType::Fire => m.immunity_fire,
                FieldDamageType::Poison => m.immunity_poison,
                FieldDamageType::Energy => m.immunity_energy,
            },
            Some(CreatureKind::Npc(_)) => true,
            _ => false,
        }
    }

    /// C2 — Check if `cid` is standing on a magic field of the given damage type.
    /// 772 `TSkillPoison/Burning/Energy::Event` scans `GetFirstObject(pos)` for AVOID items
    /// with matching `AVOIDDAMAGETYPES` (`crskill.cc:1032-1044,1065-1075,1091-1101`).
    pub(crate) fn creature_standing_on_field(
        &self,
        cid: CreatureId,
        kind: FieldDamageType,
    ) -> bool {
        let Some(creature) = self.creatures.get(cid) else {
            return false;
        };
        let pos = creature.base().position;
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        let body = tile.body();
        body.down_items
            .iter()
            .chain(body.top_items.iter())
            .any(|&iid| {
                let Some(sid) = self.items.get(iid).map(|i| i.item_type) else {
                    return false;
                };
                self.items_db.avoid_damage_type(sid) == Some(kind)
            })
    }

    /// Remove replaceable MAGICFIELD items on `pos` before placing another — TFS
    /// `Tile::addThing` (`tile.cpp:917-938`) / 772 `CreateField` (`magic.cc:1034-1041`).
    pub(crate) fn remove_replaceable_magic_fields_on_tile(&mut self, pos: Position) {
        let existing: Vec<_> = self
            .map
            .get_tile(pos)
            .map(|t| {
                t.body()
                    .down_items
                    .iter()
                    .chain(t.body().top_items.iter())
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for iid in existing {
            let Some(sid) = self.items.get(iid).map(|i| i.item_type) else {
                continue;
            };
            let Some(t) = self.items_db.items.get(&sid) else {
                continue;
            };
            if !t.is_magic_field() {
                continue;
            }
            // Default replaceable when unset — matches TFS fire/poison/energy fields.
            let replaceable = t
                .xml_attributes
                .get("replacemagicfields")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true);
            if replaceable {
                let _ = self.internal_remove_item_from_tile(pos, iid, u16::MAX);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::CreatureKind;
    use crate::cylinder::CylinderFlags;
    use crate::game_world::GameWorld;
    use crate::item::Item;
    use crate::sim_harness::{
        beat_driven_test_world, ensure_walkable_tile, insert_monster, insert_player, test_player,
        TEST_SYNTHETIC_GROUND_WP,
    };
    use std::sync::Arc;
    use tfs_rust_common::enums::ConditionType;
    use tfs_rust_content::otb::ItemType;

    fn register_fire_field(
        world: &mut GameWorld,
        server_id: u16,
        initdamage: &str,
        cycles: &str,
        skippeaceful: Option<&str>,
    ) {
        let mut db = (*world.items_db).clone();
        let mut it = ItemType {
            server_id,
            type_tag: 6,
            ..Default::default()
        };
        it.xml_attributes.insert("field".into(), "fire".into());
        it.xml_attributes
            .insert("field.initdamage".into(), initdamage.into());
        it.xml_attributes
            .insert("field.cycles".into(), cycles.into());
        if let Some(v) = skippeaceful {
            it.xml_attributes
                .insert("field.skippeaceful".into(), v.into());
        }
        db.items.insert(server_id, it);
        world.items_db = Arc::new(db);
    }

    fn has_fire(world: &GameWorld, cid: CreatureId) -> bool {
        world.creatures.get(cid).is_some_and(|c| {
            c.base()
                .active_conditions
                .iter()
                .any(|cond| cond.ctype == ConditionType::Fire)
        })
    }

    fn place_field(world: &mut GameWorld, pos: Position, server_id: u16) {
        let iid = world.items.insert(Item::new_single(server_id));
        world
            .internal_add_item_to_tile(pos, iid, CylinderFlags::NONE)
            .expect("place fire field");
    }

    fn hp(world: &GameWorld, cid: CreatureId) -> i32 {
        world.creatures.get(cid).unwrap().base().health
    }

    /// Opt-in skip: a 1487-style field without skippeaceful still hits a player.
    #[test]
    fn fire_field_without_skippeaceful_hits_player() {
        let mut world = beat_driven_test_world();
        register_fire_field(&mut world, 1487, "20", "70", None);

        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", pos));
        world.map.register_creature_at(pos, player);
        let hp_before = hp(&world, player);

        place_field(&mut world, pos, 1487);

        assert!(hp(&world, player) < hp_before, "initdamage must hit player");
        assert!(has_fire(&world, player), "fire condition must start");
    }

    /// Meaning-harmless fields skip peaceful creatures; wild monsters still take the hit.
    #[test]
    fn skippeaceful_field_skips_player_hits_wild_monster() {
        let mut world = beat_driven_test_world();
        register_fire_field(&mut world, 1500, "20", "70", Some("1"));

        let ppos = Position::new(100, 100, 7);
        let mpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let php_before = hp(&world, player);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let mhp_before = hp(&world, monster);

        place_field(&mut world, ppos, 1500);
        assert_eq!(
            hp(&world, player),
            php_before,
            "peaceful skip: HP unchanged"
        );
        assert!(
            !has_fire(&world, player),
            "peaceful skip: no Fire condition"
        );

        place_field(&mut world, mpos, 1500);
        assert!(
            hp(&world, monster) < mhp_before || has_fire(&world, monster),
            "wild monster must take initdamage and/or Fire condition"
        );
    }

    /// Player summons are peaceful (`crnonpl.cc:2295`) and skip skippeaceful fields.
    #[test]
    fn skippeaceful_field_skips_player_summon() {
        let mut world = beat_driven_test_world();
        register_fire_field(&mut world, 1500, "20", "70", Some("1"));

        let ppos = Position::new(100, 100, 7);
        let spos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, spos, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let summon = insert_monster(&mut world, "Rat", spos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(summon) {
            m.base.master = Some(player);
        }
        let hp_before = hp(&world, summon);

        place_field(&mut world, spos, 1500);
        assert_eq!(hp(&world, summon), hp_before, "player summon: HP unchanged");
        assert!(
            !has_fire(&world, summon),
            "player summon: no Fire condition"
        );
    }

    /// Native searing path: initdamage 300 + fire cycles 10 (`moveuse.dat` Damage(4,300)+Damage(64,10)).
    #[test]
    fn searing_field_init_and_cycles_hit_monster() {
        let mut world = beat_driven_test_world();
        register_fire_field(&mut world, 1506, "300", "10", None);

        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);
        if let Some(c) = world.creatures.get_mut(monster) {
            c.base_mut().health = 500;
            c.base_mut().max_health = 500;
        }
        let hp_before = hp(&world, monster);

        place_field(&mut world, pos, 1506);

        assert!(hp(&world, monster) < hp_before, "initdamage 300 must hit");
        assert!(has_fire(&world, monster), "fire cycles must start");
    }
}
