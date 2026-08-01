//! Magic-field place / step-in damage — TFS `MagicField` domain, 772 DoT outcomes.
//!
//! Domain: TFS `MagicField::onStepInField` / `MoveEvent::AddItemField` — `combat.cpp`,
//! `movement.cpp`.
//! Outcomes: 772 `DAMAGE_*_PERIODIC` → burning/poison/energy timers (`crmain.cc:582-612`,
//! `crskill.cc` TSkillBurning/Poison/Energy); field kind from items.xml `field` attr.

use tfs_rust_common::enums::{CombatType, ConditionType};
use tfs_rust_common::Position;
use tfs_rust_content::items::FieldDamageType;

use crate::combat::{apply_condition, CombatDamage, CombatParams};
use crate::condition::{ActiveCondition, ConditionData};
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
    /// Instant `initdamage` (items.xml) then DoT condition. 772 fire/energy timer lengths
    /// come from `MechanicsProfile` (10/8, 25/10); poison uses xml `cycles` as strength.
    pub(crate) fn apply_magic_field_to_creature(
        &mut self,
        target: CreatureId,
        field_server_id: u16,
    ) {
        let Some(field_kind) = self.items_db.avoid_damage_type(field_server_id) else {
            return;
        };
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

        let (ctype, combat, timer_rounds, poison_rank, skill_count, skill_max_count) =
            match field_kind {
                FieldDamageType::Fire => {
                    // 772 `SetTimer(SKILL_BURNING, Damage/10, 8, 8)` — `crmain.cc:600`.
                    // XML `field.cycles` ≈ Cycle (Events); profile `ticks` = MaxCount interval.
                    let interval = self.mechanics.profile.conditions.fire.ticks.max(1);
                    let cycle = if cycles > 0 { cycles } else { interval };
                    (
                        ConditionType::Fire,
                        CombatType::Fire,
                        Some(cycle),
                        0,
                        interval,
                        interval,
                    )
                }
                FieldDamageType::Energy => {
                    let interval = self.mechanics.profile.conditions.energy.ticks.max(1);
                    let cycle = if cycles > 0 { cycles } else { interval };
                    (
                        ConditionType::Energy,
                        CombatType::Energy,
                        Some(cycle),
                        0,
                        interval,
                        interval,
                    )
                }
                FieldDamageType::Poison => {
                    let rank = if cycles > 0 { cycles } else { 1 };
                    // 772 `SetTimer(SKILL_POISON, Damage, 3, 3, -1)` — `crmain.cc:589`.
                    (ConditionType::Poison, CombatType::Earth, None, rank, 3, 3)
                }
            };

        if init_damage > 0 {
            let snap = self.combat_notify_snapshot(target);
            let hp_before = self
                .creatures
                .get(target)
                .map(|k| k.base().health)
                .unwrap_or(0);
            let damage = CombatDamage {
                primary: (combat, -(init_damage)),
                secondary: (CombatType::Undefined, 0),
            };
            self.combat_execute_with_stimulus(None, target, &damage, &CombatParams::default());
            let hp_after = self
                .creatures
                .get(target)
                .map(|k| k.base().health)
                .unwrap_or(0);
            let damage_done = (hp_before - hp_after).max(0);
            if let Some(snap) = snap {
                self.notify_player_combat_damage(None, target, damage_done, combat, snap);
            }
        }

        // Target may have died from init damage.
        if self.creatures.get(target).is_none() {
            return;
        }

        let data = match ctype {
            ConditionType::Poison => ConditionData::Damage {
                total_rank: poison_rank.max(1),
            },
            _ => ConditionData::Generic { ticks: 0 },
        };
        let cond = ActiveCondition::new(0, 0, ctype, data, timer_rounds)
            .with_skill_timer(skill_count, skill_max_count);
        apply_condition(&mut self.creatures, target, cond);
        self.on_condition_started(target, ctype);
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
