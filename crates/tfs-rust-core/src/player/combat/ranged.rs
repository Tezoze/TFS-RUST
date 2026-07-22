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
//! Reuses `combat::math::{probe_hit, weapon_damage, armor_reduction}`, `roll_target_defense`,
//! and `combat_execute_with_stimulus` — no parallel player combat math module
//! (`tfs-code-hygiene.md`). Era knobs flow through `MechanicsProfile`/`FormulaHooks`;
//! per-vocation `formula.dist_damage` from the cached `VocationProfile` snapshot.
//!
//! Wand/rod **data** comes from `world.weapons` (`wands.lua` / `rods.lua`). Scripted ammo
//! (burst, poison arrow) uses Lua `onUseWeapon` → `Combat:execute` / conditions — same TFS
//! domain shape as `data/scripts/weapons/*.lua`.
//! Distance item stats still come from `items.xml` (`attack`, `shoot_range`, `shoot_effect`).

use tfs_rust_common::enums::CombatType;
use tfs_rust_common::Position;

use crate::combat::math::{armor_reduction, probe_hit, weapon_damage};
use crate::combat::{CombatDamage, CombatParams};
use crate::cylinder::CylinderFlags;
use crate::config::ConfigManager;
use crate::creature::{roll_target_defense, CreatureKind};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::inventory::{InventorySlot, WEAPON_DISTANCE, WEAPON_WAND};
use crate::lua_scope::fire_on_use_weapon;
use crate::monster_ai::chebyshev;
use crate::player_combat::CombatResult;

/// Fallback specials for distance ammo **without** `onUseWeapon`.
///
/// Poison/burst are Lua-owned (`poison_arrow.lua` / `burst_arrow.lua`). Keep this only for
/// packs that set `poisondamagecycles` with no script — do not add new native specials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AmmoSpecialEffect {
    None,
    /// Unscripted fallback — prefer Lua `Condition(CONDITION_POISON)` DoT.
    Poison,
}

impl AmmoSpecialEffect {
    fn from_item(_item_type: u16, poison_cycles: i32) -> Self {
        if poison_cycles > 0 {
            Self::Poison
        } else {
            Self::None
        }
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
    /// Returns `Distance` for bow+ammo or throwing weapons, `Wand` for wands/rods that pass
    /// Lua `WandDef` level/vocation gates, `None` otherwise.
    ///
    /// 772 `GetWeapon` skips `RESTRICTLEVEL` / `RESTRICTPROFESSION` items (`crcombat.cc:62-76`).
    /// Era content applies those flags to **wands/rods** (Lua `weapon:level` / `:vocation`), not
    /// ordinary melee weapons — do not gate swords/clubs here.
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
                WEAPON_WAND => {
                    if self.player_meets_wand_requirements(cid, item.item_type) {
                        return RangedArm::Wand;
                    }
                    // Underleveled / wrong vocation — skip like `GetWeapon` `continue`.
                }
                _ => {}
            }
        }
        RangedArm::None
    }

    /// Lua `WandDef` level + vocation gates — `wands.lua` / `rods.lua` via `WeaponRegistry`.
    ///
    /// Empty `vocations` map ⇒ no vocation filter. Keys are allowed names (TFS
    /// `weapon:vocation(name[, showInDescription])` — the bool is description-only; both
    /// `"Sorcerer"` and `"Master Sorcerer"` entries allow those vocations).
    pub(crate) fn player_meets_wand_requirements(&self, cid: CreatureId, wand_type_id: u16) -> bool {
        let Some(def) = self.weapons.get_wand(wand_type_id) else {
            // No Lua wand def — still treat as wand for classify; `player_wand_attack` re-arms.
            return true;
        };
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return false;
        };
        if def.level > 0 && (p.level as u32) < def.level {
            return false;
        }
        if def.vocations.is_empty() {
            return true;
        }
        let Some(voc) = self.vocations.get(p.vocation_id) else {
            return false;
        };
        def.vocations.contains_key(&voc.name)
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
        // Lua level/vocation — same gates as `classify_player_ranged_arm` (772 GetWeapon skip).
        if !self.player_meets_wand_requirements(cid, wand_item_type) {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 200);
            }
            return;
        }

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
        let damage = if variation > 0 {
            strength + self.parity_rng.random(-variation, variation)
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
            // Typed color from wand element — `crmain.cc:746-755` (Fire→ORANGE, Energy→LIGHTBLUE).
            // Must not hardcode Physical (blood-family color); that made fire/energy wands look red.
            self.notify_player_combat_damage(Some(cid), target_id, damage_done, damage_type, snap);
        }

        // Missile animation — broadcast after the damage lands (C++ `::Missile` at `:736`).
        if shoot_type != 0 {
            self.broadcast_distance_shoot(master_pos, target_pos, shoot_type);
        }

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
    /// `Fragility = THROWFRAGILITY` from Lua `weapon:breakChance` when `action("move")`
    /// (`crcombat.cc:779-784`, PC-3a).
    fn player_distance_attack(&mut self, cid: CreatureId, target_id: CreatureId) {
        let server_ms = self.server_ms;
        let profile = self.mechanics.profile;
        let hooks = &self.mechanics.hooks;

        // Resolve the weapon (bow vs throw) and ammo (for bows). `player_get_weapon(cid, false)`
        // returns the ammo item for bows, the weapon itself for throwing weapons. `active_slot`
        // is the inventory slot the consumed item lives in (Ammo for bows, hand for throwing) —
        // needed to push the slot update to the client after consumption.
        let (weapon_iid, ammo_iid, is_bow, active_slot) = self.resolve_distance_weapon(cid);
        // `HitChance` — bow=90, throw=75 (`crcombat.cc:766,779`).
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
        let (attack_value, shoot_type, special_effect, effect_strength, active_type_id) = {
            let Some(item) = self.items.get(active_iid) else {
                return;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                return;
            };
            let shoot_type = it.shoot_effect.unwrap_or(0);
            // Poison arrow: `poisondamagecycles` in xml_attributes. Burst / scripted ammo
            // use Lua `onUseWeapon` when registered (not hardcoded by id).
            let poison_cycles = it
                .xml_attributes
                .get("poisondamagecycles")
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0);
            let special = AmmoSpecialEffect::from_item(it.server_id, poison_cycles);
            let effect_strength = match special {
                AmmoSpecialEffect::Poison => poison_cycles,
                AmmoSpecialEffect::None => 0,
            };
            (it.attack, shoot_type, special, effect_strength, it.server_id)
        };
        // TFS scripted distance: `onUseWeapon` replaces native damage/specials
        // (`weapons.cpp:365-369`). Burst arrow lives here via `burst_arrow.lua`.
        let scripted = self
            .weapons
            .get_distance(active_type_id)
            .is_some_and(|d| d.has_on_use)
            || self.events.has_weapon_on_use(active_type_id);

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
                    p.skill_level(crate::player::combat::SkillNr::Distance),
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

        // `Probe(Difficulty * 15, HitChance, LearningPoints > 0)` (`crcombat.cc:793-794`).
        let hit = probe_hit(skill, difficulty * 15, hit_chance, &self.parity_rng);
        // Probe `Increase(1)` + `LearningPoints -= 1` (`crcombat.cc:795-797`).
        // On hit, `GetAttackDamage` → `ProbeValue(..., Increase)` is a **second** try
        // (`crcombat.cc:803`, `crskill.cc:535-538`) — even when Lua `onUseWeapon` owns damage.
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

        // Target armor snapshot for Physical armor inside `Damage` (`crmain.cc:624`).
        // Defense side effects only when the **attacker** wears a shield (`crcombat.cc:809-811`).
        let defense_snap = self.melee_defense_snapshot_for(target_id);
        let attacker_has_shield = self.player_get_shield(cid).is_some();

        let drop_pos; // Missile impact tile (target tile on hit, random adjacent on miss).
        if hit {
            drop_pos = target_pos;

            // `GetAttackDamage` skill side-effect — second Increase while LP still > 0.
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                if p.base.learning_points > 0 {
                    levels_gained = levels_gained.saturating_add(p.skill_increase(
                        crate::player::combat::SkillNr::Distance,
                        skill_tries,
                        &profile,
                        hooks,
                    ));
                    p.base.learning_points -= 1;
                    skill_trained = skill_trained || skill_tries > 0;
                }
            }

            // Skill notify after hooks last used on hit (native arm uses hooks below).
            if !scripted {
                // Native physical arrow/throw — skipped when `onUseWeapon` owns damage
                // (TFS `Weapon::internalUseWeapon` scripted branch).
                // Magnitude only — skill Increase already applied above (GetAttackDamage SE).
                let attack_roll =
                    weapon_damage(&profile, hooks, skill, attack_value, mode, level, &self.parity_rng);
                let attack_roll = ((attack_roll as f64) * dist_mult).floor() as i32;

                let armor_roll =
                    armor_reduction(&profile, hooks, defense_snap.armor, &self.parity_rng);
                let dmg = (attack_roll - armor_roll.max(0)).max(0);

                if attacker_has_shield {
                    let defense_gate_passed = self
                        .creatures
                        .get(target_id)
                        .is_some_and(|k| server_ms >= k.base().earliest_defend_ms);
                    let _ = match self.creatures.get_mut(target_id) {
                        Some(kind) => roll_target_defense(
                            kind.base_mut(),
                            server_ms,
                            &profile,
                            hooks,
                            defense_snap,
                            &self.parity_rng,
                        ),
                        None => return,
                    };
                    if defense_gate_passed {
                        self.player_shield_wearout(target_id);
                        self.player_shield_skill_learning(target_id, defense_snap.has_shield);
                    }
                }

                if skill_trained {
                    self.notify_skill_tries_gained(
                        cid,
                        crate::player::combat::SkillNr::Distance,
                        levels_gained,
                    );
                }

                if dmg <= 0 {
                    self.broadcast_magic_effect(target_pos, 4u8);
                }

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
                    self.notify_player_combat_damage(
                        Some(cid),
                        target_id,
                        damage_done,
                        CombatType::Physical,
                        snap,
                    );
                }

                if damage_done > 0 {
                    if let Some(k) = self.creatures.get_mut(cid) {
                        k.base_mut().activate_learning();
                    }
                }

                // Unscripted poison-ammo fallback only. Live pack uses
                // `poison_arrow.lua` `onUseWeapon` → ConditionDamage (not Earth HP).
                if special_effect == AmmoSpecialEffect::Poison && effect_strength > 0 {
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
            } else if skill_trained {
                self.notify_skill_tries_gained(
                    cid,
                    crate::player::combat::SkillNr::Distance,
                    levels_gained,
                );
            }
        } else {
            // Miss — drop the projectile on a random adjacent tile (`crcombat.cc:817-828`).
            let mut dx = 0i32;
            let mut dy = 0i32;
            if dist_x > 1 || dist_y > 1 {
                dx = self.parity_rng.random(-1, 1);
                dy = self.parity_rng.random(-1, 1);
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

        // Scripted ammo (burst arrow, etc.) — TFS `Weapon::executeUseWeapon` after missile
        // (`weapons.cpp:485`). Hit → VARIANT_NUMBER; miss → VARIANT_POSITION at drop.
        if scripted {
            if hit {
                let _ = fire_on_use_weapon(self, active_type_id, cid, Some(target_id), None, true);
            } else {
                let _ = fire_on_use_weapon(
                    self,
                    active_type_id,
                    cid,
                    None,
                    Some((drop_pos.x, drop_pos.y, drop_pos.z)),
                    false,
                );
            }
        }

        // Ammo consumption — `random(0, 99) < Fragility` → `Delete`, else `Move` to drop
        // (`crcombat.cc:844-849`). Bow/`removecount` → Fragility 100; throw+`move` → breakChance.
        let fragility = self.distance_ammo_fragility(active_iid, is_bow);
        self.consume_or_drop_ammo(cid, active_slot, active_iid, drop_pos, fragility);

        // `EFFECT_POFF` on miss (`crcombat.cc:858`).
        if !hit {
            self.broadcast_magic_effect(drop_pos, 3u8);
        }

        // `DelayAttack(2000)` — `crcombat.cc:641`. Distance weapons use the fixed 2s cadence
        // in 772 (the vocation `attackspeed` is for melee; distance cadence is `2000`).
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

    /// Resolve Fragility for the consumed ammo/throw item.
    ///
    /// Bow path always uses 100 (`crcombat.cc:766`). Throw/`WEAPON_AMMO` with Lua
    /// `action("move")` uses `break_chance`; `removecount`/`removecharge` → 100.
    fn distance_ammo_fragility(&self, active_iid: ItemId, is_bow: bool) -> u8 {
        if is_bow {
            return 100;
        }
        let Some(item) = self.items.get(active_iid) else {
            return 100;
        };
        let Some(def) = self.weapons.get_distance(item.item_type) else {
            return 100;
        };
        use tfs_rust_content::weapons::WeaponConsumeAction;
        match def.consume_action {
            WeaponConsumeAction::Move => def.break_chance.min(100),
            WeaponConsumeAction::RemoveCount | WeaponConsumeAction::RemoveCharge => 100,
        }
    }

    /// `random(0, 99) < Fragility` → Delete 1, else Move 1 to `drop_pos` (`crcombat.cc:844-849`).
    fn consume_or_drop_ammo(
        &mut self,
        cid: CreatureId,
        slot: u8,
        iid: ItemId,
        drop_pos: Position,
        fragility: u8,
    ) {
        let roll = self.parity_rng.rand_mod(100) as u8;
        if roll < fragility {
            self.decrement_inventory_item(cid, slot, iid);
            return;
        }
        // Move 1 charge to the drop tile.
        let Some(item_type) = self.items.get(iid).map(|i| i.item_type) else {
            return;
        };
        let remove_entire = if let Some(item) = self.items.get_mut(iid) {
            if item.count > 1 {
                item.count -= 1;
                false
            } else {
                true
            }
        } else {
            return;
        };
        if remove_entire {
            let _ = self.internal_remove_item_from_inventory_slot(cid, slot, iid);
            self.items.remove(iid);
            self.broadcast_player_inventory_slot(cid, slot, None);
        } else {
            self.broadcast_player_inventory_slot(cid, slot, Some(iid));
        }
        let dropped = self.items.insert(crate::item::Item::new_single(item_type));
        let _ = self.internal_add_item_to_tile(drop_pos, dropped, CylinderFlags::NO_LIMIT);
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
            let _ = self.internal_remove_item_from_inventory_slot(cid, slot, iid);
            self.items.remove(iid);
            self.broadcast_player_inventory_slot(cid, slot, None);
        } else {
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
    use crate::sim_harness::{
        beat_driven_test_world, ensure_walkable_tile, insert_monster_with_config,
        insert_spectator_player, minimal_world, sim_hero_player, TEST_SYNTHETIC_GROUND_WP,
    };
    use tfs_rust_common::{ConnId, Position};
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
        let parity = crate::sim_glibc_rand::GlibcRngState::seed(1);
        for _ in 0..100 {
            assert!(probe_hit(0, 0, 100, &parity));
            assert!(probe_hit(100, 0, 0, &parity));
        }
    }

    /// `probe_hit` with `prob == 0` never hits (the `<=` makes `rand%100 == 0` the only hit,
    /// but `prob == 0` means `rand%100 <= 0` is true only when `rand%100 == 0` — 1% chance).
    /// Use a high skill so the `skill >= rand%diff` gate always passes, isolating the prob roll.
    #[test]
    fn probe_hit_prob_zero_rarely_hits() {
        let parity = crate::sim_glibc_rand::GlibcRngState::seed(2);
        let mut hits = 0;
        for _ in 0..1000 {
            if probe_hit(1000, 100, 0, &parity) {
                hits += 1;
            }
        }
        // ~1% hit rate (binomial μ=10, σ≈3.15); allow 0..=25 to avoid flakiness.
        assert!(hits <= 25, "expected ~1% hits, got {hits}");
    }

    /// `probe_hit` with `prob == 100` always hits when the skill gate passes.
    #[test]
    fn probe_hit_prob_100_always_hits_when_skill_passes() {
        let parity = crate::sim_glibc_rand::GlibcRngState::seed(3);
        for _ in 0..1000 {
            // skill=1000, diff=100 → rand%100 is 0..99, skill >= that always → prob roll 100.
            assert!(probe_hit(1000, 100, 100, &parity));
        }
    }

    /// `probe_hit` with low skill vs high difficulty mostly misses the skill gate.
    #[test]
    fn probe_hit_low_skill_high_diff_misses() {
        let parity = crate::sim_glibc_rand::GlibcRngState::seed(4);
        let mut hits = 0;
        for _ in 0..1000 {
            // skill=0, diff=100 → rand%100 is 0..99; 0 >= rand is only true when rand==0 (1%).
            if probe_hit(0, 100, 100, &parity) {
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

    /// Fire wand animated text uses COLOR_ORANGE (198), not blood COLOR_RED (180).
    /// Regression: `player_wand_attack` previously hardcoded `CombatType::Physical` into notify.
    /// Uses 772 codec — 1098 `encode_animated_text` is a no-op.
    #[test]
    fn wand_fire_animated_text_uses_orange_not_blood_red() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        let target_pos = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, target_pos, TEST_SYNTHETIC_GROUND_WP);
        let mut player = sim_hero_player("Hero", pos);
        player.mana = 100;
        let conn = ConnId(1);
        let cid = insert_spectator_player(&mut world, conn, player);
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
            2187,
            make_wand(2187, tfs_rust_common::enums::ShootEffect::Fire as u8),
        );
        register_wand(
            &mut world,
            2187,
            WandDef {
                item_id: 2187,
                level: 13,
                mana_cost: 3,
                element: CombatType::Fire,
                damage_min: 20,
                damage_max: 20,
                ..Default::default()
            },
        );
        world.server_ms = 1000;
        world.pending_outgoing.clear();
        world.player_ranged_attack_strike(cid, target);

        const COLOR_ORANGE: u8 = 198; // crmain.cc:749 DAMAGE_FIRE
        const COLOR_RED: u8 = 180; // blood-family physical
        let pkts = world
            .pending_outgoing
            .get(&conn)
            .expect("spectator must receive packets");
        let anim = pkts
            .iter()
            .find(|b| b.len() >= 7 && b[0] == 0x84)
            .expect("must send animated damage text 0x84");
        assert_eq!(
            anim[6], COLOR_ORANGE,
            "fire wand text color must be ORANGE(198), got {} (blood red would be {})",
            anim[6],
            COLOR_RED
        );
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

    /// Unscripted fallback detection from `poisondamagecycles`; live pack uses Lua.
    #[test]
    fn ammo_special_effect_poison_detection() {
        assert_eq!(
            AmmoSpecialEffect::from_item(2545, 50),
            AmmoSpecialEffect::Poison
        );
        assert_eq!(
            AmmoSpecialEffect::from_item(2546, 0),
            AmmoSpecialEffect::None
        );
        assert_eq!(
            AmmoSpecialEffect::from_item(2544, 0),
            AmmoSpecialEffect::None
        );
    }

    /// DistanceAttack does not subtract target defense when the attacker has no shield.
    /// Target with huge defense + zero armor still takes damage; defend timestamps unchanged.
    #[test]
    fn distance_attack_no_attacker_shield_skips_defense_subtraction() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.skills.dist = 100;
        let cid = world.creatures.insert(CreatureKind::Player(player));
        let target_pos = Position::new(103, 100, 7);
        let mut cfg = crate::creature::MonsterAiConfig::default();
        cfg.defense = 500;
        cfg.armor = 0;
        cfg.melee_skill = 100;
        let target = insert_monster_with_config(&mut world, "Tank", target_pos, 100, cfg);
        let defend_before = world.creatures.get(target).unwrap().base().earliest_defend_ms;

        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2456,
            make_bow(2456, 1, 6),
        );
        let arrow_iid = world.items.insert(Item::new(2544, 10));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            let idx = crate::inventory::slot_to_array_index(InventorySlot::Ammo as u8).unwrap();
            p.equipment_slots[idx] = Some(arrow_iid);
        }
        let arrow_it = make_ammo(2544, 1, 50, tfs_rust_common::enums::ShootEffect::Arrow as u8);
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
        // Retry until a hit lands (high skill → almost always).
        let mut damaged = false;
        for i in 0..20 {
            if !world.creatures.contains_key(target) {
                damaged = true;
                break;
            }
            let hp = world.creatures.get(target).unwrap().base().health;
            if hp < 100 {
                damaged = true;
                break;
            }
            // Restock ammo if depleted.
            if world.get_player_inventory_item(cid, InventorySlot::Ammo as u8).is_none() {
                let iid = world.items.insert(Item::new(2544, 10));
                if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
                    let idx =
                        crate::inventory::slot_to_array_index(InventorySlot::Ammo as u8).unwrap();
                    p.equipment_slots[idx] = Some(iid);
                }
            }
            world.server_ms = 1000 + i * 3000;
            world.player_ranged_attack_strike(cid, target);
        }
        assert!(
            damaged || world
                .creatures
                .get(target)
                .is_some_and(|k| k.base().health < 100),
            "ranged hit must deal damage despite huge target defense (no attacker shield)"
        );
        if let Some(k) = world.creatures.get(target) {
            assert_eq!(
                k.base().earliest_defend_ms,
                defend_before,
                "defend gate must not advance without attacker shield"
            );
        }
    }

    /// Scripted ammo (`has_on_use`) skips native physical damage on hit.
    #[test]
    fn scripted_ammo_skips_native_primary_damage() {
        use tfs_rust_content::weapons::{DistanceWeaponDef, WeaponConsumeAction, WeaponRegistry};

        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let target_pos = Position::new(103, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, target_pos, TEST_SYNTHETIC_GROUND_WP);
        for x in 100..=103u16 {
            ensure_walkable_tile(
                &mut world.map,
                Position::new(x, 100, 7),
                TEST_SYNTHETIC_GROUND_WP,
            );
        }

        let mut player = sim_hero_player("Hero", pos);
        player.skills.dist = 100;
        let cid = world.creatures.insert(CreatureKind::Player(player));
        world.map.register_creature_at(pos, cid);

        let mut cfg = crate::creature::MonsterAiConfig::default();
        cfg.armor = 0;
        cfg.defense = 0;
        let target = insert_monster_with_config(&mut world, "Primary", target_pos, 100, cfg);

        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2456,
            make_bow(2456, 1, 6),
        );
        let arrow_iid = world.items.insert(Item::new(2546, 5));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            let idx = crate::inventory::slot_to_array_index(InventorySlot::Ammo as u8).unwrap();
            p.equipment_slots[idx] = Some(arrow_iid);
        }
        let burst_it = make_ammo(
            2546,
            1,
            50, // high attack — would natively deal damage if scripted path were skipped
            tfs_rust_common::enums::ShootEffect::BurstArrow as u8,
        );
        if !world.items_db.items.contains_key(&2546) {
            let mut items = std::collections::HashMap::clone(&world.items_db.items);
            items.insert(2546, burst_it);
            let client_to_server =
                std::collections::HashMap::clone(&world.items_db.client_to_server);
            world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
                items,
                client_to_server,
            });
        }

        let mut registry = WeaponRegistry::default();
        registry.distance.insert(
            2546,
            DistanceWeaponDef {
                item_id: 2546,
                consume_action: WeaponConsumeAction::RemoveCount,
                has_on_use: true,
                ..Default::default()
            },
        );
        world.weapons = std::sync::Arc::new(registry);

        world.server_ms = 1000;
        // NullEventDispatcher: onUseWeapon is a no-op; native damage must still be skipped.
        for i in 0..30 {
            if world.get_player_inventory_item(cid, InventorySlot::Ammo as u8).is_none() {
                let iid = world.items.insert(Item::new(2546, 5));
                if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
                    let idx =
                        crate::inventory::slot_to_array_index(InventorySlot::Ammo as u8).unwrap();
                    p.equipment_slots[idx] = Some(iid);
                }
            }
            if let Some(k) = world.creatures.get_mut(target) {
                k.base_mut().health = 100;
            }
            world.server_ms = 1000 + i * 3000;
            world.player_ranged_attack_strike(cid, target);
        }
        let hp = world
            .creatures
            .get(target)
            .map(|k| k.base().health)
            .unwrap_or(0);
        assert_eq!(
            hp, 100,
            "has_on_use must skip native DistanceAttack damage (hp={hp})"
        );
    }

    /// Throw with `action(move)` + `breakChance(0)` always drops 1 to the impact tile.
    #[test]
    fn throwing_break_chance_zero_moves_to_drop_tile() {
        use tfs_rust_content::weapons::{DistanceWeaponDef, WeaponConsumeAction, WeaponRegistry};

        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.skills.dist = 100;
        let cid = world.creatures.insert(CreatureKind::Player(player));
        let target_pos = Position::new(103, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, target_pos, TEST_SYNTHETIC_GROUND_WP);
        let target = insert_monster_with_config(
            &mut world,
            "Rat",
            target_pos,
            100,
            crate::creature::MonsterAiConfig::default(),
        );

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
        let mut reg = WeaponRegistry::clone(&world.weapons);
        reg.distance.insert(
            2389,
            DistanceWeaponDef {
                item_id: 2389,
                break_chance: 0,
                consume_action: WeaponConsumeAction::Move,
                ..Default::default()
            },
        );
        world.weapons = std::sync::Arc::new(reg);

        world.server_ms = 1000;
        world.player_ranged_attack_strike(cid, target);

        let spear_count = world.items.get(spear_iid).map(|i| i.count).unwrap_or(0);
        assert_eq!(spear_count, 4, "stack should decrement when moving to ground");

        // Dropped spear on target tile (hit) or adjacent (miss).
        let mut found_drop = false;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let p = Position::new(
                    (target_pos.x as i32 + dx) as u16,
                    (target_pos.y as i32 + dy) as u16,
                    target_pos.z,
                );
                if let Some(tile) = world.map.get_tile(p) {
                    for &iid in tile.body().down_items.iter().chain(tile.body().top_items.iter())
                    {
                        if world.items.get(iid).is_some_and(|i| i.item_type == 2389) {
                            found_drop = true;
                        }
                    }
                }
            }
        }
        assert!(found_drop, "breakChance=0 must Move spear onto drop tile");
    }

    /// On hit with learning open, Probe + GetAttackDamage each Increase(1) — two tries, LP−2.
    #[test]
    fn distance_hit_trains_skill_twice_when_learning() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.skills.dist = 100;
        player.skills.dist_tries = 0;
        player.base.learning_points = 2;
        let cid = world.creatures.insert(CreatureKind::Player(player));
        let target_pos = Position::new(103, 100, 7);
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
            2456,
            make_bow(2456, 1, 6),
        );
        let arrow_iid = world.items.insert(Item::new(2544, 10));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            let idx = crate::inventory::slot_to_array_index(InventorySlot::Ammo as u8).unwrap();
            p.equipment_slots[idx] = Some(arrow_iid);
        }
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
        // Retry until a hit lands (high skill → almost always).
        for _ in 0..20 {
            if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
                p.base.learning_points = 2;
                p.skills.dist_tries = 0;
            }
            if let Some(item) = world.items.get_mut(arrow_iid) {
                item.count = 10;
            }
            world.player_ranged_attack_strike(cid, target);
            let (lp, tries) = match world.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => (p.base.learning_points, p.skills.dist_tries),
                _ => (0, 0),
            };
            if tries >= 2 {
                assert_eq!(tries, 2, "Probe + GetAttackDamage → two tries");
                // DamageDone > 0 → ActivateLearning sets LP=30 after the two burns.
                assert_eq!(
                    lp, 30,
                    "ActivateLearning must refresh LearningPoints after a damaging hit"
                );
                return;
            }
        }
        panic!("expected a distance hit within 20 attempts");
    }

    /// Underleveled wand (Lua `weapon:level`) is skipped — not classified as Wand.
    #[test]
    fn underleveled_wand_not_classified() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.level = 8; // below Inferno / Cosmic Energy gate (26)
        let cid = world.creatures.insert(CreatureKind::Player(player));
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2187,
            make_wand(2187, 0),
        );
        register_wand(
            &mut world,
            2187,
            WandDef {
                item_id: 2187,
                level: 26,
                mana_cost: 13,
                element: CombatType::Fire,
                damage_min: 55,
                damage_max: 75,
                ..Default::default()
            },
        );
        assert_eq!(
            world.classify_player_ranged_arm(cid),
            RangedArm::None,
            "level gate must skip wand like GetWeapon continue"
        );
        assert_eq!(world.player_weapon_range(cid), 1);
    }

    /// Wand with vocation filter and no matching registry entry is not usable.
    #[test]
    fn wrong_vocation_wand_not_classified() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.vocation_id = 4; // knight-ish; registry empty → name lookup fails
        player.level = 50;
        let cid = world.creatures.insert(CreatureKind::Player(player));
        equip_item(
            &mut world,
            cid,
            InventorySlot::Left as u8,
            2190,
            make_wand(2190, 0),
        );
        let mut vocs = std::collections::HashMap::new();
        vocs.insert("Sorcerer".into(), true);
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
                vocations: vocs,
            },
        );
        assert_eq!(world.classify_player_ranged_arm(cid), RangedArm::None);
    }

    /// Hand/ammo equip triggers CheckCombatValues DelayAttack(2000).
    #[test]
    fn weapon_slot_change_delays_attack() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let cid = world
            .creatures
            .insert(CreatureKind::Player(sim_hero_player("Hero", pos)));
        world.server_ms = 5_000;
        world.player_maybe_delay_attack_on_weapon_slot_change(cid, InventorySlot::Left as u8);
        let earliest = world.creatures.get(cid).unwrap().base().earliest_attack_ms;
        assert_eq!(earliest, 7_000);
        // Non-weapon slots do not delay.
        world.server_ms = 8_000;
        world.player_maybe_delay_attack_on_weapon_slot_change(cid, InventorySlot::Head as u8);
        let earliest2 = world.creatures.get(cid).unwrap().base().earliest_attack_ms;
        assert_eq!(earliest2, 7_000, "head slot must not DelayAttack");
    }
}
