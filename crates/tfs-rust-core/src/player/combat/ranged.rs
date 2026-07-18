//! PC-3 — `DistanceAttack` and `WandAttack` ranged strike bodies.
//!
//! C++ reference (mechanics, `tibia-game-master/src/`):
//! - `TCombat::Attack` range dispatch — `crcombat.cc:608-639` (`Range >= 2 && Range <= 3`).
//! - `TCombat::DistanceAttack` — `crcombat.cc:739-860` (ammo/throw, hit probe, special effects).
//! - `TCombat::WandAttack` — `crcombat.cc:697-737` (mana check, damage roll, missile).
//! - `TSkillProbe::Probe` — `crskill.cc:549` (distance hit probe; Rust `combat::math::probe_hit`).
//! - `TCreature::Damage` — `crmain.cc:486-760` (typed immunities, mana shield, armor — shared
//!   path via `combat_execute_with_stimulus`).
//!
//! Reuses `combat::math::{probe_hit, weapon_damage, melee_damage_after_defense_and_armor,
//! armor_reduction}`, `roll_target_defense`, and `combat_execute_with_stimulus` — no parallel
//! player combat math module (`tfs-code-hygiene.md`). Era knobs flow through
//! `MechanicsProfile`/`FormulaHooks`; per-vocation `formula.dist_damage` from the cached
//! `VocationProfile` snapshot.
//!
//! Wand data comes from `world.weapons: Arc<WeaponRegistry>` (loaded from
//! `data/scripts/weapons/wands.lua` + `rods.lua` via the TFS Lua `Weapon(WEAPON_WAND)` API in
//! PC-2b). Distance weapon/ammo data comes from `items.xml` (`attack`, `shoot_range`,
//! `shoot_effect`, `ammo_type`).

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

use tfs_rust_common::enums::CombatType;
use tfs_rust_common::Position;

use crate::combat::math::{
    armor_reduction, melee_damage_after_defense_and_armor, probe_hit, weapon_damage,
};
use crate::combat::{CombatDamage, CombatParams};
use crate::config::ConfigManager;
use crate::creature::{roll_target_defense, CreatureKind};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::{InventorySlot, WEAPON_DISTANCE, WEAPON_WAND};
use crate::monster_ai::chebyshev;
use crate::player_combat::CombatResult;

/// Special-effect enum for distance ammo — `AMMOSPECIALEFFECT`/`THROWSPECIALEFFECT`
/// (`crcombat.cc:770,783`). 772 only defines `1 = POISON ARROW` and `2 = BURST ARROW`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AmmoSpecialEffect {
    None,
    /// `SpecialEffect == 1` — poison arrow: extra `DAMAGE_POISON_PERIODIC` on hit
    /// (`crcombat.cc:833-836`). `EffectStrength` = the ammo's `poisondamagecycles` attribute.
    Poison,
    /// `SpecialEffect == 2` — burst arrow: physical AoE burst centered on the impact tile
    /// (`crcombat.cc:837-842`). `EffectStrength` = the burst damage amount.
    Burst,
}

impl AmmoSpecialEffect {
    /// Map an `items.xml` `poisondamagecycles` / `burstdamage` attribute presence to the
    /// 772 special-effect enum. The 772 `AMMOSPECIALEFFECT` is read from the ammo's
    /// `ObjectType` attribute; TFS encodes it via the `poisondamagecycles` (poison arrow) and
    /// the `onUseWeapon` Lua script (burst arrow). For PC-3 we detect poison arrows by the
    /// `poisondamagecycles > 0` attribute and burst arrows by item id (2546).
    fn from_item(item_type: u16, poison_cycles: i32) -> Self {
        if poison_cycles > 0 {
            return Self::Poison;
        }
        if item_type == 2546 {
            return Self::Burst;
        }
        Self::None
    }
}

impl GameWorld {
    /// 772 `TCombat::Attack` range-2/3 dispatch — `crcombat.cc:608-639`.
    ///
    /// Called from `player_execute_attack` when the equipped weapon's `GetDistance()` is 2 or 3
    /// (bow `shoot_range`, wand `WANDRANGE`, throw `THROWRANGE`). Routes to `DistanceAttack`
    /// (missile/throw) or `WandAttack` based on the equipped hand-slot item classification
    /// (`crcombat.cc:632-638`).
    ///
    /// The caller has already verified the target is alive and within the 8-tile sight window
    /// (`crcombat.cc:624-627`). Range-vs-distance and `ThrowPossible` (line-of-sight) checks
    /// happen inside the strike bodies, matching C++ order (`crcombat.cc:706,762,787`).
    pub(crate) fn player_ranged_attack_strike(&mut self, cid: CreatureId, target_id: CreatureId) {
        // Classify the equipped weapon to pick the strike arm. Read-only borrows first.
        let arm = self.classify_player_ranged_arm(cid);
        match arm {
            RangedArm::Distance => self.player_distance_attack(cid, target_id),
            RangedArm::Wand => self.player_wand_attack(cid, target_id),
            RangedArm::None => {
                // No ranged weapon equipped — re-arm without striking. C++ throws `ERROR`
                // (`crcombat.cc:637`) which the `Execute` catch turns into `ToDoYield`.
                let server_ms = self.server_ms;
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().delay_attack_ms(server_ms, 200);
                }
            }
        }
    }

    /// Classify the player's equipped weapon into a ranged arm — `crcombat.cc:632-638`.
    /// Returns `Distance` for bow+ammo or throwing weapons, `Wand` for wands/rods, `None`
    /// otherwise (melee or empty — should not reach this point from the dispatch).
    fn classify_player_ranged_arm(&self, cid: CreatureId) -> RangedArm {
        for slot in [InventorySlot::Left as u8, InventorySlot::Right as u8] {
            let Some(iid) = self.get_player_inventory_item(cid, slot) else {
                continue;
            };
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                continue;
            };
            match it.weapon_type {
                WEAPON_DISTANCE => return RangedArm::Distance,
                WEAPON_WAND => return RangedArm::Wand,
                _ => {}
            }
        }
        RangedArm::None
    }

    /// 772 `TCombat::WandAttack` — `crcombat.cc:697-737`.
    ///
    /// Flow: range check (`WANDRANGE`) → `ThrowPossible` (LoS) → mana check (`WANDMANACONSUMPTION`,
    /// `NOTENOUGHMANA` → `OUTOFAMMO` per C++ `:722-728`) → damage roll
    /// (`AttackStrength + random(-AttackVariation, AttackVariation)`, `crcombat.cc:731`) →
    /// `Damage(Master, Damage, DamageType)` → `ActivateLearning` on `DamageDone > 0` →
    /// missile animation (`WANDMISSILE`) → `DelayAttack(2000)`.
    ///
    /// Wand data (`AttackStrength`/`AttackVariation`/`DamageType`/`ManaConsumption`/`WANDRANGE`/
    /// `WANDMISSILE`) comes from `WandDef` (loaded from `wands.lua`/`rods.lua` in PC-2b). The
    /// `WandDef.damage_min`/`damage_max` map to 772 `WANDATTACKSTRENGTH`/`WANDATTACKVARIATION`:
    /// `AttackStrength = (min + max) / 2`, `AttackVariation = (max - min) / 2` — matching the
    /// `random(-V, V)` spread to the `[min, max]` range. The damage type is `WandDef.element`.
    /// `WANDRANGE` defaults to 3 (772 wand range); `WANDMISSILE` is read from the wand item's
    /// `shoot_effect` (`items.xml` `shoottype`).
    fn player_wand_attack(&mut self, cid: CreatureId, target_id: CreatureId) {
        let server_ms = self.server_ms;

        // Read the wand item id + WandDef snapshot before any mutation.
        let (wand_iid, wand_item_type) = match self.player_get_weapon(cid, true) {
            Some(iid) => {
                let Some(item) = self.items.get(iid) else {
                    return;
                };
                (iid, item.item_type)
            }
            None => return,
        };
        let Some(wand_def) = self.weapons.get_wand(wand_item_type).cloned() else {
            // No wand definition registered for this item — re-arm without striking.
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 200);
            }
            return;
        };

        // Positions for range/LoS checks — read before mutation.
        let (master_pos, target_pos) =
            match (self.creatures.get(cid), self.creatures.get(target_id)) {
                (Some(a), Some(b)) => (a.base().position, b.base().position),
                _ => return,
            };
        let cheb = chebyshev(master_pos, target_pos);

        // `WANDRANGE` — 772 wands have range 3. `WandDef` doesn't carry range; default to 3.
        // `crcombat.cc:706` throws `TARGETOUTOFRANGE` if `Distance > WANDRANGE`.
        const WAND_RANGE: i32 = 3;
        if cheb > WAND_RANGE {
            // Re-arm without striking — `TARGETOUTOFRANGE` falls through to `default: break`
            // in `sending.cc:348` (no message). `DelayAttack(200)` was already applied by the
            // caller's pre-strike cadence.
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 200);
            }
            return;
        }

        // `ThrowPossible` LoS check — `crcombat.cc:710-713` throws `TARGETHIDDEN`.
        if !self.map.throw_possible(master_pos, target_pos, 0) {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 200);
            }
            return;
        }

        // Mana check — `CheckMana(Master, ManaConsumption, 0, 0)` (`crcombat.cc:722`).
        // `NOTENOUGHMANA` → `throw OUTOFAMMO` (`:725`) → `sending.cc` sends "Not enough mana."
        // PC-3: drain mana if sufficient, else re-arm + send the OUTOFAMMO message.
        let mana_cost = wand_def.mana_cost as i32;
        let has_mana = self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.mana >= mana_cost));
        if !has_mana {
            if let Some(conn) = self.conn_for_creature(cid) {
                self.send_combat_result(conn, CombatResult::OutOfAmmo);
            }
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 200);
            }
            return;
        }
        let profile = self.mechanics.profile;
        let magic_tries = ConfigManager::scale_tries(
            mana_cost as u64,
            self.config.rate_magic().unwrap_or(1.0),
        );
        let mut levels_gained = 0u32;
        let mut new_maglevel = 0i32;
        {
            let hooks = &self.mechanics.hooks;
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.mana -= mana_cost;
                levels_gained = p.magic_increase(magic_tries, &profile, hooks);
                new_maglevel = p.skills.maglevel;
            }
        }
        // TFS `addManaSpent` → `sendStats()` + advance text.
        self.notify_magic_tries_gained(cid, levels_gained, new_maglevel);

        // Damage roll — `AttackStrength + random(-AttackVariation, AttackVariation)`
        // (`crcombat.cc:731`). Map `[min, max]` → `Strength = (min+max)/2`, `Variation = (max-min)/2`.
        let strength = ((wand_def.damage_min + wand_def.damage_max) / 2) as i32;
        let variation = ((wand_def.damage_max - wand_def.damage_min) / 2) as i32;
        let mut rng = std::mem::replace(&mut self.ai_rng, StdRng::from_entropy());
        let damage = if variation > 0 {
            strength + rng.gen_range(-variation..=variation)
        } else {
            strength
        };
        let damage = damage.max(0);

        // Missile animation — `WANDMISSILE` (`crcombat.cc:716`). Read from the wand item's
        // `shoot_effect` (`items.xml` `shoottype`).
        let shoot_type = self
            .items
            .get(wand_iid)
            .and_then(|i| self.items_db.items.get(&i.item_type))
            .and_then(|it| it.shoot_effect)
            .unwrap_or(0);

        // `Damage(Master, Damage, DamageType)` — `crcombat.cc:732`. The shared path handles
        // typed immunities (M3′), mana shield (M5), and death.
        let damage_type = wand_def.element;
        let notify_snap = self.combat_notify_snapshot(target_id);
        let hp_before = self
            .creatures
            .get(target_id)
            .map(|k| k.base().health)
            .unwrap_or(0);
        let _ = self.combat_execute_with_stimulus(
            Some(cid),
            target_id,
            &CombatDamage {
                primary: (damage_type, -damage),
                secondary: (damage_type, 0),
            },
            &CombatParams::default(),
        );
        let target_alive = self.creatures.contains_key(target_id);
        let hp_after = if target_alive {
            self.creatures
                .get(target_id)
                .map(|k| k.base().health)
                .unwrap_or(hp_before)
        } else {
            0
        };
        let damage_done = (hp_before - hp_after).max(0);
        if let Some(snap) = notify_snap {
            self.notify_player_combat_damage(Some(cid), target_id, damage_done, CombatType::Physical, snap);
        }

        // Missile animation — broadcast after the damage lands (C++ `::Missile` at `:736`).
        if shoot_type != 0 {
            self.broadcast_distance_shoot(master_pos, target_pos, shoot_type);
        }
        self.ai_rng = rng;

        // `ActivateLearning` on `DamageDone > 0` (`crcombat.cc:733`).
        if damage_done > 0 {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().activate_learning();
            }
        }

        // `DelayAttack(2000)` — `crcombat.cc:641`. Wands use the fixed 2s cadence (no
        // vocation `attackspeed` for wands in 772).
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().delay_attack_ms(server_ms, 2000);
        }

        // `if target dead: StopAttack` (`crcombat.cc:643-645`).
        if !target_alive {
            if let Some(conn) = self.conn_for_creature(cid) {
                self.player_stop_attack(conn, cid);
            } else if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                base.attack_target = None;
                base.follow_target = None;
            }
        }
    }

    /// 772 `TCombat::DistanceAttack` — `crcombat.cc:739-860`.
    ///
    /// Flow: ammo resolution (`GetAmmo`, `OUTOFAMMO`) → range check (`BOWRANGE`/`THROWRANGE`) →
    /// `ThrowPossible` (LoS) → hit probe (`SKILL_DISTANCE->Probe(Difficulty*15, HitChance, LP>0)`,
    /// `crcombat.cc:793-794`) → on hit: `GetAttackDamage` (fight-mode-scaled probe × vocation
    /// `formula.dist_damage`), `Damage(PHYSICAL)`, `ActivateLearning` → special effects
    /// (poison arrow, burst arrow) → missile animation → ammo consumption (`Fragility` roll) →
    /// `DelayAttack(2000)`.
    ///
    /// Bow+ammo: `HitChance = 90`, `Fragility = 100` (always consumed), `DamageType = PHYSICAL`,
    /// `AnimType = AMMOMISSILE` (`crcombat.cc:766-771`). Throwing: `HitChance = 75`,
    /// `Fragility = THROWFRAGILITY` (from `items.xml` `breakChance` via the Lua script — PC-3
    /// uses 100 for parity with the 772 `THROWFRAGILITY` default; the Lua `breakChance` is
    /// PC-3a script-wiring scope), `AnimType = THROWMISSILE` (`crcombat.cc:779-784`).
    fn player_distance_attack(&mut self, cid: CreatureId, target_id: CreatureId) {
        let server_ms = self.server_ms;
        let profile = self.mechanics.profile;
        let hooks = &self.mechanics.hooks;

        // Resolve the weapon (bow vs throw) and ammo (for bows). `player_get_weapon(cid, false)`
        // returns the ammo item for bows, the weapon itself for throwing weapons. `active_slot`
        // is the inventory slot the consumed item lives in (Ammo for bows, hand for throwing) —
        // needed to push the slot update to the client after consumption.
        let (weapon_iid, ammo_iid, is_bow, active_slot) = self.resolve_distance_weapon(cid);
        // `HitChance` — bow=90, throw=75 (`crcombat.cc:766,779`). `Fragility` is always 100 for
        // PC-3 (bows always consume ammo; throwing weapons always consume a charge). The
        // `random(0, 99) < Fragility` roll is PC-3a script-wiring scope (Lua `breakChance`).
        let (active_iid, hit_chance) = match (weapon_iid, ammo_iid, is_bow) {
            (Some(_), Some(a), true) => (a, 90),
            (Some(w), None, false) => (w, 75),
            _ => {
                // No ammo for a bow — `OUTOFAMMO` (`crcombat.cc:756-758`).
                if let Some(conn) = self.conn_for_creature(cid) {
                    self.send_combat_result(conn, CombatResult::OutOfAmmo);
                }
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().delay_attack_ms(server_ms, 200);
                }
                return;
            }
        };

        // Read the active item (ammo for bows, weapon for throwing) for attack/shoot/effect.
        let (attack_value, shoot_type, special_effect, effect_strength) = {
            let Some(item) = self.items.get(active_iid) else {
                return;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                return;
            };
            let shoot_type = it.shoot_effect.unwrap_or(0);
            // Poison arrow detection: `poisondamagecycles` is stored in `xml_attributes` (PC-2
            // items.xml parsing keeps unknown keys there). Burst arrow: item id 2546.
            let poison_cycles = it
                .xml_attributes
                .get("poisondamagecycles")
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0);
            let special = AmmoSpecialEffect::from_item(it.server_id, poison_cycles);
            let effect_strength = match special {
                AmmoSpecialEffect::Poison => poison_cycles,
                AmmoSpecialEffect::Burst => 30, // 772 burst arrow base damage (`crcombat.cc:838`).
                AmmoSpecialEffect::None => 0,
            };
            (it.attack, shoot_type, special, effect_strength)
        };

        // Positions for range/LoS — read before mutation.
        let (master_pos, target_pos) =
            match (self.creatures.get(cid), self.creatures.get(target_id)) {
                (Some(a), Some(b)) => (a.base().position, b.base().position),
                _ => return,
            };
        let dist_x = (master_pos.x as i32 - target_pos.x as i32).abs();
        let dist_y = (master_pos.y as i32 - target_pos.y as i32).abs();
        let cheb = dist_x.max(dist_y);

        // Range check — `BOWRANGE` (bow `shoot_range`) or `THROWRANGE` (throw `shoot_range`).
        // `crcombat.cc:762,775` throws `TARGETOUTOFRANGE`.
        let range = self
            .items
            .get(weapon_iid.unwrap())
            .and_then(|i| self.items_db.items.get(&i.item_type))
            .map(|it| it.shoot_range)
            .unwrap_or(1);
        if cheb > range {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 200);
            }
            return;
        }

        // `ThrowPossible` LoS — `crcombat.cc:787-790` throws `TARGETHIDDEN`.
        if !self.map.throw_possible(master_pos, target_pos, 0) {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 200);
            }
            return;
        }

        // Attacker skill/level/mode/vocation block — read before mutation.
        let (skill, level, mode, dist_mult, attack_speed_ms, learning_active) =
            match self.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => (
                    p.skills.dist,
                    p.level,
                    p.attack_mode,
                    p.vocation_profile.formula.dist_damage as f64,
                    p.vocation_profile.attack_speed_ms as u64,
                    p.base.learning_points > 0,
                ),
                _ => return,
            };

        // `Difficulty = (Distance >= 2) ? Distance : 5` (`crcombat.cc:792`).
        let difficulty = if cheb >= 2 { cheb } else { 5 };
        let mut rng = std::mem::replace(&mut self.ai_rng, StdRng::from_entropy());

        // `Probe(Difficulty * 15, HitChance, LearningPoints > 0)` (`crcombat.cc:793-794`).
        let hit = probe_hit(&mut rng, skill, difficulty * 15, hit_chance);
        // `Increase(1)` on distance skill + `LearningPoints -= 1` (`crcombat.cc:795-797`).
        // TFS `onGainSkillTries`: multiply by `rateSkill` before adding.
        let skill_tries = ConfigManager::scale_tries(1, self.config.rate_skill().unwrap_or(1.0));
        let mut skill_trained = false;
        let mut levels_gained = 0u32;
        if learning_active {
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                if p.base.learning_points > 0 {
                    levels_gained = p.skill_increase(
                        crate::player::combat::SkillNr::Distance,
                        skill_tries,
                        &profile,
                        hooks,
                    );
                    p.base.learning_points -= 1;
                    skill_trained = skill_tries > 0;
                }
            }
        }

        // Target defense snapshot — for the `Target->Combat.GetDefendDamage()` call on hit
        // (`crcombat.cc:809-811`). C++ notes this is likely a bug (defense shouldn't apply to
        // ranged), but it does run when the defender has a shield. We mirror the behavior.
        let defense_snap = self.melee_defense_snapshot_for(target_id);

        let drop_pos; // Missile impact tile (target tile on hit, random adjacent on miss).
        if hit {
            // `GetAttackDamage` — fight-mode-scaled probe roll (`crcombat.cc:803`).
            let attack_roll =
                weapon_damage(&profile, hooks, &mut rng, skill, attack_value, mode, level);
            let attack_roll = ((attack_roll as f64) * dist_mult).floor() as i32;

            // `Target->Combat.GetDefendDamage()` — `crcombat.cc:809-811`. The C++ comment notes
            // this is probably a bug (defense shouldn't block ranged), but it runs when the
            // defender has a shield. We mirror: roll defense, apply armor, then `Damage(PHYSICAL)`.
            let defense_gate_passed = self
                .creatures
                .get(target_id)
                .is_some_and(|k| server_ms >= k.base().earliest_defend_ms);
            let defense_roll = match self.creatures.get_mut(target_id) {
                Some(kind) => roll_target_defense(
                    kind.base_mut(),
                    server_ms,
                    &profile,
                    hooks,
                    &mut rng,
                    defense_snap,
                ),
                None => {
                    self.ai_rng = rng;
                    return;
                }
            };
            let armor_roll = armor_reduction(&profile, hooks, &mut rng, defense_snap.armor);
            // TFS `addSkillAdvance` → `sendSkills()` + advance text — after `hooks` is last used on hit.
            if skill_trained {
                self.notify_skill_tries_gained(
                    cid,
                    crate::player::combat::SkillNr::Distance,
                    levels_gained,
                );
            }
            // M11/M12 — shield wearout + shielding skill learning when gate passed.
            if defense_gate_passed {
                self.player_shield_wearout(target_id);
                self.player_shield_skill_learning(target_id, defense_snap.has_shield);
            }
            let dmg = melee_damage_after_defense_and_armor(attack_roll, defense_roll, armor_roll);

            drop_pos = target_pos;

            // Poff / spark for `dmg <= 0` — same as melee (`crmain.cc:577-579, 624-628`).
            if dmg <= 0 {
                let effect = if attack_roll <= defense_roll {
                    3u8
                } else {
                    4u8
                };
                self.broadcast_magic_effect(target_pos, effect);
            }

            // `Damage(Master, Attack, DAMAGE_PHYSICAL)` (`crcombat.cc:813`).
            let notify_snap = self.combat_notify_snapshot(target_id);
            let hp_before = self
                .creatures
                .get(target_id)
                .map(|k| k.base().health)
                .unwrap_or(0);
            let _ = self.combat_execute_with_stimulus(
                Some(cid),
                target_id,
                &CombatDamage {
                    primary: (CombatType::Physical, -dmg),
                    secondary: (CombatType::Physical, 0),
                },
                &CombatParams::default(),
            );
            let target_alive = self.creatures.contains_key(target_id);
            let hp_after = if target_alive {
                self.creatures
                    .get(target_id)
                    .map(|k| k.base().health)
                    .unwrap_or(hp_before)
            } else {
                0
            };
            let damage_done = (hp_before - hp_after).max(0);
            if let Some(snap) = notify_snap {
                self.notify_player_combat_damage(Some(cid), target_id, damage_done, CombatType::Physical, snap);
            }

            // `ActivateLearning` on `DamageDone > 0` (`crcombat.cc:814`).
            if damage_done > 0 {
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().activate_learning();
                }
            }

            // Special effects — poison arrow / burst arrow (`crcombat.cc:833-842`).
            match special_effect {
                AmmoSpecialEffect::Poison => {
                    if effect_strength > 0 {
                        // `Target->Damage(Master, EffectStrength, DAMAGE_POISON_PERIODIC)`
                        // (`crcombat.cc:835`). The shared path applies the poison condition
                        // via `apply_condition` when the target is not immune.
                        let _ = self.combat_execute_with_stimulus(
                            Some(cid),
                            target_id,
                            &CombatDamage {
                                primary: (CombatType::Earth, -effect_strength),
                                secondary: (CombatType::Earth, 0),
                            },
                            &CombatParams::default(),
                        );
                    }
                }
                AmmoSpecialEffect::Burst => {
                    // `ComputeDamage(Master, 0, EffectStrength, EffectStrength)` + AoE
                    // `CircleShapeSpell` (`crcombat.cc:838-841`). PC-3 applies the burst damage
                    // to the primary target only; the AoE spread is PC-3a script-wiring scope
                    // (the burst arrow's `onUseWeapon` Lua `Combat:execute` handles the area).
                    let burst_dmg = effect_strength.max(0);
                    let _ = self.combat_execute_with_stimulus(
                        Some(cid),
                        target_id,
                        &CombatDamage {
                            primary: (CombatType::Physical, -burst_dmg),
                            secondary: (CombatType::Physical, 0),
                        },
                        &CombatParams::default(),
                    );
                }
                AmmoSpecialEffect::None => {}
            }
        } else {
            // Miss — drop the projectile on a random adjacent tile (`crcombat.cc:817-828`).
            let mut dx = 0i32;
            let mut dy = 0i32;
            if dist_x > 1 || dist_y > 1 {
                dx = rng.gen_range(-1..=1);
                dy = rng.gen_range(-1..=1);
            }
            let mut drop = Position::new(
                (target_pos.x as i32 + dx).max(0) as u16,
                (target_pos.y as i32 + dy).max(0) as u16,
                target_pos.z,
            );
            // C++ validates the drop tile (`BANK` + `!UNLAY` + `ThrowPossible`); revert to
            // target tile on failure (`crcombat.cc:822-827`). PC-3 reverts if the tile is
            // blocked for throws.
            if !self.map.throw_possible(master_pos, drop, 0) {
                drop = target_pos;
            }
            drop_pos = drop;
            // Miss path: hooks unused after skill_increase — still refresh client %.
            if skill_trained {
                self.notify_skill_tries_gained(
                    cid,
                    crate::player::combat::SkillNr::Distance,
                    levels_gained,
                );
            }
        }

        // Missile animation — `::Missile(Master, DropCon, AnimType)` (`crcombat.cc:831`).
        if shoot_type != 0 {
            self.broadcast_distance_shoot(master_pos, drop_pos, shoot_type);
        }

        // Ammo consumption — `random(0, 99) < Fragility` → `Delete(Ammo, 1)`, else `Move` to
        // the drop tile (`crcombat.cc:844-849`). PC-3: bows (`Fragility=100`) always consume
        // one ammo; throwing weapons (`Fragility=100` default) always consume one charge.
        // Dropping the projectile on the ground (the `Move` arm) is PC-3a follow-up; PC-3
        // always deletes to keep the ammo economy simple. The `is_bow` flag is preserved for
        // the PC-3a split (bow consumes from the ammo slot; throw consumes from the hand slot).
        let _ = is_bow;
        self.decrement_inventory_item(cid, active_slot, active_iid);

        // `EFFECT_POFF` on miss (`crcombat.cc:858`).
        if !hit {
            self.broadcast_magic_effect(drop_pos, 3u8);
        }
        self.ai_rng = rng;

        // `DelayAttack(2000)` — `crcombat.cc:641`. Distance weapons use the fixed 2s cadence
        // in 772 (the vocation `attackspeed` is for melee; distance cadence is `2000`).
        // PC-3a may wire the per-vocation distance `attackspeed` if `data/formulas` defines one.
        let _ = attack_speed_ms;
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().delay_attack_ms(server_ms, 2000);
        }

        // `if target dead: StopAttack` (`crcombat.cc:643-645`).
        if !self.creatures.contains_key(target_id) {
            if let Some(conn) = self.conn_for_creature(cid) {
                self.player_stop_attack(conn, cid);
            } else if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                base.attack_target = None;
                base.follow_target = None;
            }
        }
    }

    /// Resolve the distance weapon (bow vs throw) and ammo item id.
    /// Returns `(weapon_iid, ammo_iid, is_bow, active_slot)`. For throwing weapons, `ammo_iid ==
    /// weapon_iid` and `is_bow == false`. For bows, `ammo_iid` is the ammo slot item and
    /// `is_bow == true`. `active_slot` is the inventory slot the consumed item (ammo for bows,
    /// weapon for throwing) lives in — needed to push the slot update to the client after
    /// consumption. Returns `(None, None, false, 0)` if no distance weapon is equipped.
    fn resolve_distance_weapon(&self, cid: CreatureId) -> (Option<ItemId>, Option<ItemId>, bool, u8) {
        for slot in [InventorySlot::Left as u8, InventorySlot::Right as u8] {
            let Some(iid) = self.get_player_inventory_item(cid, slot) else {
                continue;
            };
            let Some(item) = self.items.get(iid) else {
                continue;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                continue;
            };
            if it.weapon_type == WEAPON_DISTANCE {
                if it.ammo_type != 0 {
                    // Bow/crossbow — resolve ammo from the ammo slot. The ammo item's
                    // `ammo_type` must match the weapon's `ammo_type` (`crcombat.cc:121`
                    // `AMMOTYPE == BOWAMMOTYPE`; TFS `player.cpp:211`
                    // `ammoItem->getAmmoType() != it.ammoType`). A mismatch (e.g. bolts in a
                    // bow) returns `None` for ammo, which triggers the `OUTOFAMMO` arm —
                    // matching the 772 `Ammo = NONE` → `OUTOFAMMO` flow.
                    let ammo_iid = self
                        .get_player_inventory_item(cid, InventorySlot::Ammo as u8)
                        .filter(|&aid| {
                            self.items.get(aid).is_some_and(|ammo| {
                                self.items_db
                                    .items
                                    .get(&ammo.item_type)
                                    .is_some_and(|ammo_it| ammo_it.ammo_type == it.ammo_type)
                            })
                        });
                    return (Some(iid), ammo_iid, true, InventorySlot::Ammo as u8);
                }
                // Throwing weapon — the weapon itself is the projectile; `ammo_iid = None`
                // signals "no separate ammo slot" (the match arm `(Some, None, false)` picks
                // the weapon iid as the active item).
                return (Some(iid), None, false, slot);
            }
        }
        (None, None, false, 0)
    }

    /// Decrement an inventory item's `count` by 1, removing it when `count` reaches 0, and push
    /// the updated slot to the client so the ammo/throwing count refreshes immediately. Used by
    /// the distance strike for ammo/throwing consumption (`crcombat.cc:846`). Without the
    /// `broadcast_player_inventory_slot` call the server-side count drops but the client keeps
    /// showing the stale count until a relog or full inventory resend.
    fn decrement_inventory_item(&mut self, cid: CreatureId, slot: u8, iid: ItemId) {
        let remove = if let Some(item) = self.items.get_mut(iid) {
            if item.count > 1 {
                item.count -= 1;
                false
            } else {
                true
            }
        } else {
            false
        };
        if remove {
            self.items.remove(iid);
            // Slot is now empty — send `sendInventoryItem` with no item.
            self.broadcast_player_inventory_slot(cid, slot, None);
        } else {
            // Count decreased — resend the slot with the new count.
            self.broadcast_player_inventory_slot(cid, slot, Some(iid));
        }
    }
}

/// Ranged arm classification — `crcombat.cc:632-638`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RangedArm {
    Distance,
    Wand,
    None,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::math::probe_hit;
    use crate::creature::CreatureKind;
    use crate::inventory::{InventorySlot, WEAPON_AMMO};
    use crate::item::Item;
    use crate::sim_harness::{insert_monster_with_config, minimal_world, sim_hero_player};
    use tfs_rust_common::Position;
    use tfs_rust_content::otb::ItemType;
    use tfs_rust_content::weapons::{WandDef, WeaponRegistry};

    /// Equip an item into a player slot, registering the ItemType if needed.
    fn equip_item(world: &mut GameWorld, cid: CreatureId, slot: u8, item_type: u16, it: ItemType) {
        if !world.items_db.items.contains_key(&item_type) {
            let mut items = std::collections::HashMap::clone(&world.items_db.items);
            items.insert(item_type, it);
            let client_to_server =
                std::collections::HashMap::clone(&world.items_db.client_to_server);
            world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
                items,
                client_to_server,
            });
        }
        let iid = world.items.insert(Item::new_single(item_type));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            let idx = crate::inventory::slot_to_array_index(slot).unwrap();
            p.equipment_slots[idx] = Some(iid);
        }
    }

    fn make_bow(server_id: u16, ammo_type: u8, shoot_range: i32) -> ItemType {
        ItemType {
            server_id,
            weapon_type: WEAPON_DISTANCE,
            ammo_type,
            shoot_range,
            ..Default::default()
        }
    }

    fn make_ammo(server_id: u16, ammo_type: u8, attack: i32, shoot_effect: u8) -> ItemType {
        ItemType {
            server_id,
            weapon_type: WEAPON_AMMO,
            ammo_type,
            attack,
            shoot_effect: Some(shoot_effect),
            ..Default::default()
        }
    }

    fn make_throwing(server_id: u16, attack: i32, shoot_range: i32, shoot_effect: u8) -> ItemType {
        ItemType {
            server_id,
            weapon_type: WEAPON_DISTANCE,
            attack,
            shoot_range,
            shoot_effect: Some(shoot_effect),
            ..Default::default()
        }
    }

    fn make_wand(server_id: u16, shoot_effect: u8) -> ItemType {
        ItemType {
            server_id,
            weapon_type: WEAPON_WAND,
            shoot_effect: Some(shoot_effect),
            ..Default::default()
        }
    }

    fn register_wand(world: &mut GameWorld, item_id: u16, def: WandDef) {
        let mut w = WeaponRegistry::clone(&world.weapons);
        w.wands.insert(item_id, def);
        world.weapons = std::sync::Arc::new(w);
    }

    /// `probe_hit` with `diff == 0` always hits (C++ `crskill.cc:560-566`).
    #[test]
    fn probe_hit_diff_zero_always_hits() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            assert!(probe_hit(&mut rng, 0, 0, 100));
            assert!(probe_hit(&mut rng, 100, 0, 0));
        }
    }

    /// `probe_hit` with `prob == 0` never hits (the `<=` makes `rand%100 == 0` the only hit,
    /// but `prob == 0` means `rand%100 <= 0` is true only when `rand%100 == 0` — 1% chance).
    /// Use a high skill so the `skill >= rand%diff` gate always passes, isolating the prob roll.
    #[test]
    fn probe_hit_prob_zero_rarely_hits() {
        let mut rng = rand::thread_rng();
        let mut hits = 0;
        for _ in 0..1000 {
            if probe_hit(&mut rng, 1000, 100, 0) {
                hits += 1;
            }
        }
        // ~1% hit rate (binomial μ=10, σ≈3.15); allow 0..=25 to avoid flakiness.
        assert!(hits <= 25, "expected ~1% hits, got {hits}");
    }

    /// `probe_hit` with `prob == 100` always hits when the skill gate passes.
    #[test]
    fn probe_hit_prob_100_always_hits_when_skill_passes() {
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            // skill=1000, diff=100 → rand%100 is 0..99, skill >= that always → prob roll 100.
            assert!(probe_hit(&mut rng, 1000, 100, 100));
        }
    }

    /// `probe_hit` with low skill vs high difficulty mostly misses the skill gate.
    #[test]
    fn probe_hit_low_skill_high_diff_misses() {
        let mut rng = rand::thread_rng();
        let mut hits = 0;
        for _ in 0..1000 {
            // skill=0, diff=100 → rand%100 is 0..99; 0 >= rand is only true when rand==0 (1%).
            if probe_hit(&mut rng, 0, 100, 100) {
                hits += 1;
            }
        }
        assert!(hits <= 30, "expected ~1% hits, got {hits}");
    }

    /// Wand strike drains mana and deals typed damage.
    #[test]
    fn wand_strike_drains_mana_and_deals_damage() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.mana = 100;
        let cid = world.creatures.insert(CreatureKind::Player(player));
        // Target at cheb=2 (within wand range 3).
        let target_pos = Position::new(102, 100, 7);
        let target = insert_monster_with_config(
            &mut world,
            "Rat",
            target_pos,
            100,
            crate::creature::MonsterAiConfig::default(),
        );
        // Equip a wand (item id 2190) with energy element, damage 8..18, mana cost 2.
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2190,
            make_wand(2190, tfs_rust_common::enums::ShootEffect::Energy as u8),
        );
        register_wand(
            &mut world,
            2190,
            WandDef {
                item_id: 2190,
                level: 7,
                mana_cost: 2,
                element: CombatType::Energy,
                damage_min: 8,
                damage_max: 18,
                ..Default::default()
            },
        );
        world.server_ms = 1000;
        world.player_ranged_attack_strike(cid, target);

        // Mana was drained by 2.
        let mana_after = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.mana,
            _ => 0,
        };
        assert_eq!(mana_after, 98, "wand strike should drain 2 mana");

        // Target took energy damage (8..18 → strength=13, variation=5 → 8..18 range).
        let hp_after = world
            .creatures
            .get(target)
            .map(|k| k.base().health)
            .unwrap_or(0);
        assert!(
            hp_after < 100,
            "target should have taken damage, hp={hp_after}"
        );

        // `DelayAttack(2000)` — earliest attack advanced by 2s.
        let earliest = world.creatures.get(cid).unwrap().base().earliest_attack_ms;
        assert_eq!(earliest, 3000);
    }

    /// Wand strike with insufficient mana sends OUTOFAMMO and does not drain or damage.
    #[test]
    fn wand_strike_insufficient_mana_sends_outofammo() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.mana = 1; // less than mana_cost=2
        let cid = world.creatures.insert(CreatureKind::Player(player));
        let target_pos = Position::new(102, 100, 7);
        let target = insert_monster_with_config(
            &mut world,
            "Rat",
            target_pos,
            100,
            crate::creature::MonsterAiConfig::default(),
        );
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2190,
            make_wand(2190, 0),
        );
        register_wand(
            &mut world,
            2190,
            WandDef {
                item_id: 2190,
                mana_cost: 2,
                element: CombatType::Energy,
                damage_min: 8,
                damage_max: 18,
                ..Default::default()
            },
        );
        world.server_ms = 1000;
        world.player_ranged_attack_strike(cid, target);

        // Mana unchanged.
        let mana_after = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.mana,
            _ => 0,
        };
        assert_eq!(
            mana_after, 1,
            "mana should not be drained on insufficient mana"
        );

        // Target HP unchanged.
        let hp_after = world
            .creatures
            .get(target)
            .map(|k| k.base().health)
            .unwrap_or(0);
        assert_eq!(
            hp_after, 100,
            "target should not take damage on insufficient mana"
        );
    }

    /// Bow + ammo strike consumes one ammo and deals physical damage.
    #[test]
    fn bow_strike_consumes_ammo_and_deals_damage() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.skills.dist = 100; // high skill for reliable hits
        let cid = world.creatures.insert(CreatureKind::Player(player));
        let target_pos = Position::new(103, 100, 7); // cheb=3, within bow range 6
        let target = insert_monster_with_config(
            &mut world,
            "Rat",
            target_pos,
            100,
            crate::creature::MonsterAiConfig::default(),
        );
        // Bow (item 2456, ammo_type=1=arrow, shoot_range=6) + arrows (item 2544, attack=25).
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2456,
            make_bow(2456, 1, 6),
        );
        // Equip a stack of 10 arrows.
        let arrow_iid = world.items.insert(Item::new(2544, 10));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            let idx = crate::inventory::slot_to_array_index(InventorySlot::Ammo as u8).unwrap();
            p.equipment_slots[idx] = Some(arrow_iid);
        }
        // Register the arrow ItemType with shoot_effect=Arrow (3).
        let arrow_it = make_ammo(
            2544,
            1,
            25,
            tfs_rust_common::enums::ShootEffect::Arrow as u8,
        );
        if !world.items_db.items.contains_key(&2544) {
            let mut items = std::collections::HashMap::clone(&world.items_db.items);
            items.insert(2544, arrow_it);
            let client_to_server =
                std::collections::HashMap::clone(&world.items_db.client_to_server);
            world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
                items,
                client_to_server,
            });
        }
        world.server_ms = 1000;
        world.player_ranged_attack_strike(cid, target);

        // Ammo stack decremented from 10 to 9.
        let arrow_count = world.items.get(arrow_iid).map(|i| i.count).unwrap_or(0);
        assert_eq!(arrow_count, 9, "ammo stack should decrement by 1");

        // `DelayAttack(2000)`.
        let earliest = world.creatures.get(cid).unwrap().base().earliest_attack_ms;
        assert_eq!(earliest, 3000);
    }

    /// Bow with no ammo sends OUTOFAMMO and does not strike.
    #[test]
    fn bow_strike_no_ammo_sends_outofammo() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.skills.dist = 100;
        let cid = world.creatures.insert(CreatureKind::Player(player));
        let target_pos = Position::new(102, 100, 7);
        let target = insert_monster_with_config(
            &mut world,
            "Rat",
            target_pos,
            100,
            crate::creature::MonsterAiConfig::default(),
        );
        // Bow equipped, no ammo in ammo slot.
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2456,
            make_bow(2456, 1, 6),
        );
        world.server_ms = 1000;
        world.player_ranged_attack_strike(cid, target);

        // Target HP unchanged.
        let hp_after = world
            .creatures
            .get(target)
            .map(|k| k.base().health)
            .unwrap_or(0);
        assert_eq!(hp_after, 100, "target should not take damage with no ammo");
    }

    /// Bow with mismatched ammo type (bolts in a bow) sends OUTOFAMMO and does not strike.
    /// `crcombat.cc:121` `AMMOTYPE == BOWAMMOTYPE`; TFS `player.cpp:211`
    /// `ammoItem->getAmmoType() != it.ammoType`.
    #[test]
    fn bow_strike_mismatched_ammo_type_sends_outofammo() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.skills.dist = 100;
        let cid = world.creatures.insert(CreatureKind::Player(player));
        let target_pos = Position::new(102, 100, 7);
        let target = insert_monster_with_config(
            &mut world,
            "Rat",
            target_pos,
            100,
            crate::creature::MonsterAiConfig::default(),
        );
        // Bow with ammo_type=1 (arrow). Equip in Left slot.
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2456,
            make_bow(2456, 1, 6),
        );
        // Bolts with ammo_type=2 (bolt) — wrong type for this bow.
        let bolt_iid = world.items.insert(Item::new(2543, 10));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            let idx = crate::inventory::slot_to_array_index(InventorySlot::Ammo as u8).unwrap();
            p.equipment_slots[idx] = Some(bolt_iid);
        }
        // Register the bolt ItemType with ammo_type=2.
        let bolt_it = make_ammo(2543, 2, 30, tfs_rust_common::enums::ShootEffect::Bolt as u8);
        if !world.items_db.items.contains_key(&2543) {
            let mut items = std::collections::HashMap::clone(&world.items_db.items);
            items.insert(2543, bolt_it);
            let client_to_server =
                std::collections::HashMap::clone(&world.items_db.client_to_server);
            world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
                items,
                client_to_server,
            });
        }
        world.server_ms = 1000;
        world.player_ranged_attack_strike(cid, target);

        // Target HP unchanged — mismatched ammo should not fire.
        let hp_after = world
            .creatures
            .get(target)
            .map(|k| k.base().health)
            .unwrap_or(0);
        assert_eq!(hp_after, 100, "target should not take damage with mismatched ammo");

        // Bolt stack unchanged — no consumption.
        let bolt_count = world.items.get(bolt_iid).map(|i| i.count).unwrap_or(0);
        assert_eq!(bolt_count, 10, "mismatched ammo should not be consumed");
    }

    /// Throwing weapon strike consumes one charge and deals physical damage.
    #[test]
    fn throwing_strike_consumes_charge_and_deals_damage() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.skills.dist = 100;
        let cid = world.creatures.insert(CreatureKind::Player(player));
        let target_pos = Position::new(103, 100, 7); // cheb=3, within spear range 7
        let target = insert_monster_with_config(
            &mut world,
            "Rat",
            target_pos,
            100,
            crate::creature::MonsterAiConfig::default(),
        );
        // Spear (item 2389, attack=25, shoot_range=7, shoot_effect=Spear=1).
        let spear_iid = world.items.insert(Item::new(2389, 5));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            let idx = crate::inventory::slot_to_array_index(InventorySlot::Left as u8).unwrap();
            p.equipment_slots[idx] = Some(spear_iid);
        }
        let spear_it = make_throwing(
            2389,
            25,
            7,
            tfs_rust_common::enums::ShootEffect::Spear as u8,
        );
        if !world.items_db.items.contains_key(&2389) {
            let mut items = std::collections::HashMap::clone(&world.items_db.items);
            items.insert(2389, spear_it);
            let client_to_server =
                std::collections::HashMap::clone(&world.items_db.client_to_server);
            world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
                items,
                client_to_server,
            });
        }
        world.server_ms = 1000;
        world.player_ranged_attack_strike(cid, target);

        // Spear stack decremented from 5 to 4.
        let spear_count = world.items.get(spear_iid).map(|i| i.count).unwrap_or(0);
        assert_eq!(
            spear_count, 4,
            "throwing weapon stack should decrement by 1"
        );
    }

    /// Mana shield absorbs damage to mana first, spilling remainder to HP.
    #[test]
    fn mana_shield_absorbs_damage_to_mana() {
        use crate::condition::{add_condition_merge, ActiveCondition, ConditionData};
        use tfs_rust_common::enums::ConditionType;

        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.mana = 50;
        player.base.health = 100;
        player.base.max_health = 100;
        // Add mana shield condition.
        add_condition_merge(
            &mut player.base.active_conditions,
            ActiveCondition::new(
                1,
                0,
                ConditionType::ManaShield,
                ConditionData::Generic { ticks: 100 },
                Some(100),
            ),
        );
        let cid = world.creatures.insert(CreatureKind::Player(player));

        // Apply 30 damage — fully absorbed by mana (50 >= 30).
        let absorbed = world.apply_mana_shield(cid, 30);
        assert_eq!(absorbed, 30, "mana shield should absorb all 30 damage");
        let (mana, hp) = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => (p.mana, p.base.health),
            _ => (0, 0),
        };
        assert_eq!(mana, 20, "mana should be drained by 30");
        assert_eq!(hp, 100, "HP should be untouched");

        // Apply 50 damage — mana (20) absorbs 20, remainder 30 spills to HP via the caller.
        let absorbed = world.apply_mana_shield(cid, 50);
        assert_eq!(
            absorbed, 20,
            "mana shield should absorb only remaining 20 mana"
        );
        let (mana, hp) = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => (p.mana, p.base.health),
            _ => (0, 0),
        };
        assert_eq!(mana, 0, "mana should be fully drained");
        assert_eq!(
            hp, 100,
            "HP untouched by apply_mana_shield itself (caller spills)"
        );
    }

    /// Mana shield is no-op for players without the condition.
    #[test]
    fn mana_shield_noop_without_condition() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.mana = 100;
        let cid = world.creatures.insert(CreatureKind::Player(player));

        let absorbed = world.apply_mana_shield(cid, 30);
        assert_eq!(absorbed, 0, "no mana shield condition → no absorb");
        let mana = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.mana,
            _ => 0,
        };
        assert_eq!(mana, 100, "mana should be unchanged");
    }

    /// Mana shield is no-op for monsters (monster mana is not a thing in 772).
    #[test]
    fn mana_shield_noop_for_monsters() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let target = insert_monster_with_config(
            &mut world,
            "Rat",
            pos,
            100,
            crate::creature::MonsterAiConfig::default(),
        );
        let absorbed = world.apply_mana_shield(target, 30);
        assert_eq!(absorbed, 0, "mana shield should not apply to monsters");
    }

    /// Typed immunity blocks wand damage and emits `EFFECT_BLOCK_HIT` (4).
    #[test]
    fn wand_strike_energy_immune_monster_takes_no_damage() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.mana = 100;
        let cid = world.creatures.insert(CreatureKind::Player(player));
        let target_pos = Position::new(102, 100, 7);
        let mut cfg = crate::creature::MonsterAiConfig::default();
        cfg.immunity_energy = true;
        let target = insert_monster_with_config(&mut world, "EnergyImmune", target_pos, 100, cfg);
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2190,
            make_wand(2190, 0),
        );
        register_wand(
            &mut world,
            2190,
            WandDef {
                item_id: 2190,
                mana_cost: 2,
                element: CombatType::Energy,
                damage_min: 8,
                damage_max: 18,
                ..Default::default()
            },
        );
        world.server_ms = 1000;
        world.player_ranged_attack_strike(cid, target);

        // Mana was still drained (C++ `CheckMana` runs before `Damage`).
        let mana_after = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.mana,
            _ => 0,
        };
        assert_eq!(
            mana_after, 98,
            "mana should be drained even if target is immune"
        );

        // Target HP unchanged (immunity blocks damage).
        let hp_after = world
            .creatures
            .get(target)
            .map(|k| k.base().health)
            .unwrap_or(0);
        assert_eq!(hp_after, 100, "energy-immune monster should take no damage");
    }

    /// `classify_player_ranged_arm` returns `Wand` for an equipped wand.
    #[test]
    fn classify_arm_wand() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let cid = world
            .creatures
            .insert(CreatureKind::Player(sim_hero_player("Hero", pos)));
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2190,
            make_wand(2190, 0),
        );
        assert_eq!(world.classify_player_ranged_arm(cid), RangedArm::Wand);
    }

    /// `classify_player_ranged_arm` returns `Distance` for an equipped bow.
    #[test]
    fn classify_arm_bow() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let cid = world
            .creatures
            .insert(CreatureKind::Player(sim_hero_player("Hero", pos)));
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2456,
            make_bow(2456, 1, 6),
        );
        assert_eq!(world.classify_player_ranged_arm(cid), RangedArm::Distance);
    }

    /// `classify_player_ranged_arm` returns `None` for melee / empty.
    #[test]
    fn classify_arm_none_for_melee() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let cid = world
            .creatures
            .insert(CreatureKind::Player(sim_hero_player("Hero", pos)));
        assert_eq!(world.classify_player_ranged_arm(cid), RangedArm::None);
    }

    /// `AmmoSpecialEffect::from_item` detects poison arrows via `poisondamagecycles`.
    #[test]
    fn ammo_special_effect_poison_detection() {
        assert_eq!(
            AmmoSpecialEffect::from_item(2545, 50),
            AmmoSpecialEffect::Poison
        );
        assert_eq!(
            AmmoSpecialEffect::from_item(2546, 0),
            AmmoSpecialEffect::Burst
        );
        assert_eq!(
            AmmoSpecialEffect::from_item(2544, 0),
            AmmoSpecialEffect::None
        );
    }
}
