//! PC-2 — `CloseAttack` melee strike body (extracted from `player/combat/mod.rs`).
//!
//! C++ reference (mechanics, `tibia-game-master/src/`):
//! - `TCombat::CloseAttack` — `crcombat.cc:647-664`.
//! - `TCombat::GetAttackDamage` — `crcombat.cc:220-235` (fight-mode + `ProbeValue`).
//! - `TCombat::GetDefendDamage` — `crcombat.cc:236-274` (gate + `ProbeValue`).
//! - `TCombat::GetArmorStrength` — `crcombat.cc:286-307` (applied inside `Damage(PHYSICAL)`).
//! - `TCombat::ActivateLearning` — `crcombat.cc:526` (`LearningPoints = 30` on `DamageDone > 0`).
//! - `TCombat::DelayAttack` — `crcombat.cc:523` (`200` before, `attackspeed` after).
//! - `TSkillProbe::ProbeValue` — `crskill.cc:535` (`Increase(1)` + `LearningPoints--` while > 0).
//!
//! Reuses `combat::math::weapon_damage`, `roll_target_defense`, and
//! `combat_execute_with_stimulus` — no parallel player combat math module
//! (`tfs-code-hygiene.md`). Era knobs flow through `MechanicsProfile`/`FormulaHooks`;
//! per-vocation `formula.melee_damage` from the cached `VocationProfile` snapshot.

use tfs_rust_common::enums::CombatType;

use crate::combat::math::weapon_damage;
use crate::combat::{CombatDamage, CombatParams};
use crate::creature::{CreatureKind, roll_target_defense};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};

impl GameWorld {
    /// 772 `TCombat::CloseAttack` — `crcombat.cc:647-664`. Melee strike at `GetDistance()==1`.
    ///
    /// Flow: `DelayAttack(200)` → `GetAttackDamage` (fight-mode-scaled probe × vocation
    /// `formula.melee_damage`) → `target.GetDefendDamage` (gate + probe) → `Damage(PHYSICAL)`
    /// (armor inside the shared path) → `if DamageDone>0: ActivateLearning()` → weapon wearout
    /// → `DelayAttack(attackspeed)` → re-arm `TDAttack` → `if target dead: StopAttack`.
    ///
    /// `ProbeValue` side-effects: the attacker's `LearningPoints` decrements after the attack
    /// probe while > 0 (`crskill.cc:549`). The `Increase(1)` skill-exp accumulation (per-skill
    /// tries counters + `req_skill_tries` leveling) is PC-5 scope (§0.5 — `PlayerSkills` has no
    /// `_tries` fields yet); PC-2 wires the `LearningPoints` window + `ActivateLearning` that
    /// gates it. The target-side defense probe `LearningPoints`/shield-skill `Increase` is
    /// likewise PC-5 (no tries counters + `roll_target_defense` has no `learning` flag yet).
    pub(crate) fn player_close_attack_strike(&mut self, cid: CreatureId, target_id: CreatureId) {
        let server_ms = self.server_ms;
        let profile = self.mechanics.profile;

        // `GetAttackValue` (PC-1) + attacker mode/level — skill read **after** ProbeValue Increase.
        let (atk_value, atk_skill_nr) = self.player_get_attack_value(cid);
        let (level, mode, melee_mult, attack_speed_ms, learning_active) =
            match self.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => (
                    p.level,
                    p.attack_mode,
                    // N1 — vocation `formula.melee_damage` is 1098-only (`DamageFormula::Modern`).
                    if matches!(
                        self.mechanics.profile.damage_formula,
                        crate::formulas::DamageFormula::Modern
                    ) {
                        p.vocation_profile.formula.melee_damage as f64
                    } else {
                        1.0
                    },
                    // M3 — Use `combat::math::attack_speed_ms` so `MechanicsProfile` /
                    // Tier-2 `getAttackSpeed` hook is respected (`crcombat.cc:641`).
                    crate::combat::math::attack_speed_ms(
                        &profile,
                        &self.mechanics.hooks,
                        p.vocation_profile.attack_speed_ms as i32,
                    ) as u64,
                    p.base.learning_points > 0,
                ),
                _ => return,
            };

        // Target defense/armor snapshot — world-aware so a player target contributes
        // shield/weapon defend + shielding skill + armor (`melee_defense_snapshot_for`).
        let mut defense_snap = self.melee_defense_snapshot_for(target_id);

        // `DelayAttack(200)` before the strike (`crcombat.cc:608`).
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().delay_attack_ms(server_ms, 200);
        }

        // `ProbeValue(..., Increase)` — Increase(1) **before** Get() (`crskill.cc:535-544`).
        let skill_tries = profile.combat_skill_tries(self.config.rate_skill().unwrap_or(1.0));
        let mut skill_trained = false;
        let mut levels_gained = 0u32;
        let attack_roll = {
            let hooks = &self.mechanics.hooks;
            if learning_active
                && let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid)
                && p.base.learning_points > 0
            {
                levels_gained = p.skill_increase(atk_skill_nr, skill_tries, &profile, hooks);
                p.base.learning_points -= 1;
                skill_trained = skill_tries > 0;
            }
            let skill = match self.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => p.skill_level_profile(atk_skill_nr, &profile),
                _ => return,
            };
            // `GetAttackDamage` — fight-mode-scaled probe roll (`crcombat.cc:220`).
            let attack_roll = weapon_damage(
                &profile,
                hooks,
                skill,
                atk_value,
                mode,
                level,
                &self.parity_rng,
            );
            ((attack_roll as f64) * melee_mult).floor() as i32
        };

        // Defender shield ProbeValue Increase before defense Get() (`crcombat.cc:259-263`).
        let defense_gate_passed = self
            .creatures
            .get(target_id)
            .is_some_and(|k| server_ms >= k.base().earliest_defend_ms);
        if defense_gate_passed && defense_snap.has_shield {
            self.player_shield_skill_learning(target_id, true);
            defense_snap = self.melee_defense_snapshot_for(target_id);
        }

        // `target.GetDefendDamage` — gate + probe (`crcombat.cc:236`).
        let hooks = &self.mechanics.hooks;
        let defense_roll = match self.creatures.get_mut(target_id) {
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

        // H1 — Armor is now applied in the shared path (`combat_execute_with_stimulus`) after
        // PvP-half/absorb, matching C++ `crmain.cc:624-630`. Pass the raw armor value via
        // `CombatParams`; the caller no longer pre-rolls.
        // TFS `addSkillAdvance` → `sendSkills()` + advance text — after `hooks` is last used.
        if skill_trained {
            self.notify_skill_tries_gained(cid, atk_skill_nr, levels_gained);
        }
        // M11 — Shield wearout: decrement the defender's shield `REMAININGUSES` when the defense
        // gate passed and the defender has a chargeable shield equipped (`crcombat.cc:265-281`).
        // Player-only (monsters don't have shields). Called after `hooks` is last used to avoid
        // borrow conflict with `&mut self`.
        if defense_gate_passed {
            self.player_shield_wearout(target_id);
        }
        let dmg = (attack_roll - defense_roll).max(0);

        // Poff / spark effects — C++ `TCreature::Damage` (`crmain.cc:577-579, 624-628`):
        // `Damage <= 0` (defense >= attack) → `EFFECT_POFF` (3); physical + armor absorbs all
        // → `EFFECT_BLOCK_HIT` (4). Blood + animated text for `dmg > 0` is handled by
        // `combat_execute_with_stimulus` (`apply_physical_hit_blood`) + `notify_player_combat_damage`.
        if dmg <= 0 {
            let target_pos = self.creatures.get(target_id).map(|k| k.base().position);
            if let Some(pos) = target_pos {
                let effect = if attack_roll <= defense_roll {
                    3u8
                } else {
                    4u8
                };
                self.broadcast_magic_effect(pos, effect);
            }
        }

        // Capture the notify snapshot BEFORE `combat_execute_with_stimulus` — that path may kill
        // the target (`apply_creature_death` → `remove_creature`), making `self.creatures.get`
        // return `None`. Without this, the killing-blow damage text + health bar are never sent.
        let notify_snap = self.combat_notify_snapshot(target_id);
        let hp_before = self
            .creatures
            .get(target_id)
            .map(|k| k.base().health)
            .unwrap_or(0);
        let damage_scalar = self.combat_execute_with_stimulus(
            Some(cid),
            target_id,
            &CombatDamage {
                primary: (CombatType::Physical, -dmg),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams {
                armor: Some(defense_snap.armor),
                ..CombatParams::default()
            },
        );
        // M2 — `combat_execute_with_stimulus` returns the real `Damage` scalar (C++ `return
        // Damage`), including mana-shield absorb. Use it for `ActivateLearning` and damage
        // text instead of the HP delta (which is 0 when mana absorbs everything).
        let damage_done = damage_scalar;
        // `combat_execute_with_stimulus` calls `apply_creature_death` when HP ≤ 0, which
        // removes the target from `world.creatures`. So a missing key means the target died
        // on this strike — `hp_after = 0` so the `StopAttack` branch below fires
        // (`crcombat.cc:656`).
        let target_alive = self.creatures.contains_key(target_id);
        let _hp_after = if target_alive {
            self.creatures
                .get(target_id)
                .map(|k| k.base().health)
                .unwrap_or(hp_before)
        } else {
            0
        };
        if let Some(snap) = notify_snap {
            self.notify_player_combat_damage(
                Some(cid),
                target_id,
                damage_done,
                CombatType::Physical,
                snap,
            );
        }

        // `if (DamageDone > 0) ActivateLearning()` (`crcombat.cc:655`).
        if damage_done > 0
            && let Some(k) = self.creatures.get_mut(cid)
        {
            k.base_mut().activate_learning();
        }

        // Weapon wearout (`REMAININGUSES`) — `crcombat.cc:662`. Decrement the equipped weapon's
        // remaining uses when `ItemType.charges > 0`. Full wearout (init `count = charges` at
        // equip + remove-on-zero) is a follow-up; the decrement hook is wired here.
        self.player_strike_weapon_wearout(cid);

        // `DelayAttack(attackspeed)` after the strike (`crcombat.cc:640`, vocation `attackspeed`).
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().delay_attack_ms(server_ms, attack_speed_ms);
        }

        // `if target dead: StopAttack` (`crcombat.cc:656` + `:513-522`).
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

    /// Weapon wearout for the player strike — `crcombat.cc:662` `RemainingUses--`.
    ///
    /// Decrements the equipped weapon's `count` when `ItemType.charges > 0` (chargeable
    /// weapons model remaining uses via `count`). No-op for standard weapons (`charges == 0`).
    /// When `count` would reach 0, remove and destroy the item (772 `WearOutTarget` transform
    /// is unavailable in the TFS data pack — Delete is the domain-shaped outcome).
    fn player_strike_weapon_wearout(&mut self, cid: CreatureId) {
        let weapon_iid = match self.player_get_weapon(cid, true) {
            Some(iid) => iid,
            None => return,
        };
        self.player_chargeable_item_wearout(cid, weapon_iid);
    }

    /// M11 — Shield wearout for the defender — `crcombat.cc:265-281` `RemainingUses--`.
    ///
    /// Decrements the defender's shield `count` when `ItemType.charges > 0`. When `count`
    /// would reach 0, remove and destroy (no WearOutTarget in TFS content). Player-only.
    pub(crate) fn player_shield_wearout(&mut self, cid: CreatureId) {
        let shield_iid = match self.player_get_shield(cid) {
            Some(iid) => iid,
            None => return,
        };
        self.player_chargeable_item_wearout(cid, shield_iid);
    }

    /// Shared chargeable wearout — decrement `count`, or destroy when the last charge is spent.
    fn player_chargeable_item_wearout(&mut self, cid: CreatureId, iid: ItemId) {
        let has_charges = {
            let Some(item) = self.items.get(iid) else {
                return;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                return;
            };
            it.charges > 0
        };
        if !has_charges {
            return;
        }
        let slot = self.equipment_slot_for_item(cid, iid);
        let destroy = if let Some(item) = self.items.get_mut(iid) {
            if item.count > 1 {
                item.count -= 1;
                false
            } else {
                true
            }
        } else {
            return;
        };
        let Some(slot) = slot else {
            return;
        };
        if destroy {
            let _ = self.internal_remove_item_from_inventory_slot(cid, slot, iid);
            self.items.remove(iid);
            self.broadcast_player_inventory_slot(cid, slot, None);
            // M1 — 772 `CheckCombatValues()` on wearout-destroy → `DelayAttack(2000)`
            // (`crcombat.cc:276-278` shield, `:689-691` weapon). Refreshes `last_combat_weapons`
            // so a later swap is detected, and applies the 2s post-break attack delay.
            self.player_check_combat_values(cid);
        } else {
            self.broadcast_player_inventory_slot(cid, slot, Some(iid));
        }
    }

    /// M12 — Shielding skill `Increase(1)` when defending with a shield and learning is active.
    ///
    /// C++ `GetDefendDamage` (`crcombat.cc:259-263`): `Increase = (Shield != NONE &&
    /// LearningPoints > 0)` → `ProbeValue(..., Increase)` → `LearningPoints--`.
    /// Call **before** reading defend skill for the probe so Increase precedes Get (audit B5).
    pub(crate) fn player_shield_skill_learning(&mut self, cid: CreatureId, has_shield: bool) {
        if !has_shield {
            return;
        }
        let profile = self.mechanics.profile;
        let skill_tries = profile.combat_skill_tries(self.config.rate_skill().unwrap_or(1.0));
        let hooks = &self.mechanics.hooks;
        let mut skill_trained = false;
        let mut levels_gained = 0u32;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid)
            && p.base.learning_points > 0
        {
            levels_gained = p.skill_increase(
                crate::player::combat::SkillNr::Shielding,
                skill_tries,
                &profile,
                hooks,
            );
            p.base.learning_points -= 1;
            skill_trained = skill_tries > 0;
        }
        if skill_trained {
            self.notify_skill_tries_gained(
                cid,
                crate::player::combat::SkillNr::Shielding,
                levels_gained,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::{CreatureKind, MonsterAiConfig};
    use crate::inventory::{InventorySlot, WEAPON_SHIELD, WEAPON_SWORD};
    use crate::item::Item;
    use crate::sim_harness::{
        beat_driven_test_world, ensure_walkable_tile, insert_monster_with_config,
        insert_spectator_player, minimal_world, sim_hero_player,
    };
    use tfs_rust_common::{ConnId, Position};
    use tfs_rust_content::otb::ItemType;

    fn adjacent_pos(p: Position) -> Position {
        Position::new(p.x + 1, p.y, p.z)
    }

    /// Equip a shield (defense=22) on the defender's Right slot for snapshot tests.
    fn equip_shield(world: &mut GameWorld, cid: CreatureId) {
        let server_id = 2513u16;
        let it = ItemType {
            server_id,
            weapon_type: WEAPON_SHIELD,
            defense: 22,
            ..Default::default()
        };
        if !world.items_db.items.contains_key(&server_id) {
            let mut items = std::collections::HashMap::clone(&world.items_db.items);
            items.insert(server_id, it);
            let client_to_server =
                std::collections::HashMap::clone(&world.items_db.client_to_server);
            world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
                items,
                client_to_server,
            });
        }
        let iid = world.items.insert(Item::new_single(server_id));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(cid) {
            let idx = crate::inventory::slot_to_array_index(InventorySlot::Right as u8).unwrap();
            p.equipment_slots[idx] = Some(iid);
        }
    }

    #[test]
    fn strike_cadence_advances_earliest_attack() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let player = world
            .creatures
            .insert(CreatureKind::Player(sim_hero_player("Hero", pos)));
        let cfg = MonsterAiConfig::default();
        let target = insert_monster_with_config(&mut world, "Rat", adjacent_pos(pos), 100, cfg);
        world.server_ms = 1000;
        world.player_close_attack_strike(player, target);
        // `DelayAttack(200)` then `DelayAttack(attack_speed_ms=2000)` → 1000 + 2000 = 3000.
        let earliest = world
            .creatures
            .get(player)
            .unwrap()
            .base()
            .earliest_attack_ms;
        assert_eq!(earliest, 3000);
    }

    #[test]
    fn strike_fist_fallback_runs_without_panic() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        // No weapon equipped → fist fallback (attack=7, SKILL_FIST).
        let player = world
            .creatures
            .insert(CreatureKind::Player(sim_hero_player("Hero", pos)));
        let cfg = MonsterAiConfig::default();
        let target = insert_monster_with_config(&mut world, "Rat", adjacent_pos(pos), 100, cfg);
        world.server_ms = 500;
        world.player_close_attack_strike(player, target);
        let earliest = world
            .creatures
            .get(player)
            .unwrap()
            .base()
            .earliest_attack_ms;
        assert_eq!(earliest, 2500); // 500 + 2000
    }

    #[test]
    fn strike_zero_melee_multiplier_deals_no_damage() {
        // Vocation `formula.melee_damage = 0` floors the rolled attack to 0 → no damage, ever.
        // Deterministic: the multiplier is applied post-probe, so the rng is irrelevant here.
        // N1 — multiplier only applies under `DamageFormula::Modern` (1098).
        let mut world = minimal_world();
        world.mechanics.profile.damage_formula = crate::formulas::DamageFormula::Modern;
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.vocation_profile.formula.melee_damage = 0.0;
        player.skills.fist = 100;
        player.sim_melee_attack = 100;
        let pid = world.creatures.insert(CreatureKind::Player(player));
        let mut cfg = MonsterAiConfig::default();
        cfg.defense = 0;
        cfg.armor = 0;
        let target = insert_monster_with_config(&mut world, "Rat", adjacent_pos(pos), 100, cfg);
        let hp_before = world.creatures.get(target).unwrap().base().health;
        for i in 0..5 {
            world.server_ms = i * 3000;
            world.player_close_attack_strike(pid, target);
        }
        let hp_after = world.creatures.get(target).unwrap().base().health;
        assert_eq!(
            hp_after, hp_before,
            "zero melee_damage multiplier must deal no damage"
        );
        // No damage → ActivateLearning never fires → learning_points stays 0.
        let lp = world.creatures.get(pid).unwrap().base().learning_points;
        assert_eq!(lp, 0);
    }

    #[test]
    fn strike_kills_target_stops_attack_and_activates_learning() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.skills.fist = 100;
        player.sim_melee_attack = 5000; // huge fist attack → ~99.99% kill chance per strike
        player.vocation_profile.formula.melee_damage = 1.0;
        let pid = world.creatures.insert(CreatureKind::Player(player));
        let mut cfg = MonsterAiConfig::default();
        cfg.defense = 0;
        cfg.armor = 0;
        let target = insert_monster_with_config(&mut world, "Rat", adjacent_pos(pos), 100, cfg);
        // 1 HP target.
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(target) {
            m.base.health = 1;
            m.base.max_health = 1;
        }
        // Arm attack_target so StopAttack has something to clear on kill.
        if let Some(k) = world.creatures.get_mut(pid) {
            k.base_mut().attack_target = Some(target);
        }
        // Loop strikes until the target dies (fresh from_entropy rng per strike).
        let mut died = false;
        for i in 0..20 {
            if !world.creatures.contains_key(target) {
                died = true;
                break;
            }
            world.server_ms = i * 3000;
            world.player_close_attack_strike(pid, target);
        }
        if !world.creatures.contains_key(target) {
            died = true;
        }
        assert!(died, "target should die within 20 strikes");
        // StopAttack cleared the attacker's attack_target on kill.
        let at = world.creatures.get(pid).unwrap().base().attack_target;
        assert!(
            at.is_none(),
            "attack_target must be cleared after target death"
        );
        // ActivateLearning fired on the killing blow (DamageDone > 0 → LearningPoints = 30).
        let lp = world.creatures.get(pid).unwrap().base().learning_points;
        assert_eq!(lp, 30);
    }

    #[test]
    fn strike_player_target_snapshot_uses_shield_defense() {
        // PC-2: a player target contributes shield/weapon defend + shielding skill via
        // `melee_defense_snapshot_for` (was fist-only stub before PC-2).
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let defender = sim_hero_player("Def", adjacent_pos(pos));
        let defender_id = world.creatures.insert(CreatureKind::Player(defender));
        // Without a shield → fist fallback (sim_melee_defense=5, SKILL_FIST=10).
        let snap = world.melee_defense_snapshot_for(defender_id);
        assert_eq!(snap.defense_value, 5);
        assert_eq!(snap.defense_skill, 10); // fist skill
        // Equip a shield → shield defense takes priority + shielding skill is used.
        equip_shield(&mut world, defender_id);
        let snap2 = world.melee_defense_snapshot_for(defender_id);
        assert_eq!(snap2.defense_value, 22); // shield defense
        assert_eq!(snap2.defense_skill, 10); // shielding skill (sim_hero defaults to 10)
    }

    #[test]
    fn strike_learning_points_decrement_gated_on_active() {
        // The attack-probe LearningPoints decrement only fires when learning was already
        // active (LearningPoints > 0 before the strike). With 0 it stays 0 (no damage path
        // can ActivateLearning here because melee_damage multiplier is 0).
        let mut world = minimal_world();
        world.mechanics.profile.damage_formula = crate::formulas::DamageFormula::Modern;
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.vocation_profile.formula.melee_damage = 0.0; // no damage → no ActivateLearning
        player.base.learning_points = 5;
        let pid = world.creatures.insert(CreatureKind::Player(player));
        let cfg = MonsterAiConfig::default();
        let target = insert_monster_with_config(&mut world, "Rat", adjacent_pos(pos), 100, cfg);
        world.server_ms = 0;
        world.player_close_attack_strike(pid, target);
        // One decrement from the attack probe (learning was active); no ActivateLearning
        // (zero multiplier → DamageDone = 0).
        let lp = world.creatures.get(pid).unwrap().base().learning_points;
        assert_eq!(lp, 4);
    }

    /// Bug fix: the killing-blow damage text must be sent even when the target dies.
    /// `combat_execute_with_stimulus` calls `apply_creature_death` → `remove_creature` when
    /// HP ≤ 0, so `notify_player_combat_damage` can't read the target from `self.creatures`
    /// after the strike. The pre-captured `CombatNotifySnapshot` fixes this.
    #[test]
    fn strike_killing_blow_sends_damage_text() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(
            &mut world.map,
            pos,
            crate::sim_harness::TEST_SYNTHETIC_GROUND_WP,
        );
        ensure_walkable_tile(
            &mut world.map,
            adjacent_pos(pos),
            crate::sim_harness::TEST_SYNTHETIC_GROUND_WP,
        );
        let conn = ConnId(1);
        let mut player = sim_hero_player("Hero", pos);
        player.skills.fist = 100;
        player.sim_melee_attack = 5000;
        player.vocation_profile.formula.melee_damage = 1.0;
        let pid = insert_spectator_player(&mut world, conn, player);
        let mut cfg = MonsterAiConfig::default();
        cfg.defense = 0;
        cfg.armor = 0;
        let target = insert_monster_with_config(&mut world, "Rat", adjacent_pos(pos), 100, cfg);
        // 1 HP target — any hit kills it.
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(target) {
            m.base.health = 1;
            m.base.max_health = 1;
        }
        world.server_ms = 1000;
        world.pending_outgoing.clear();
        // Loop strikes until the target dies (fresh from_entropy rng per strike).
        let mut died = false;
        for i in 0..20 {
            if !world.creatures.contains_key(target) {
                died = true;
                break;
            }
            world.server_ms = i * 3000;
            world.pending_outgoing.clear();
            world.player_close_attack_strike(pid, target);
        }
        if !world.creatures.contains_key(target) {
            died = true;
        }
        assert!(died, "target should die within 20 strikes");

        // The animated damage text (0x84) must have been sent to the spectator (attacker)
        // on the killing blow — even though the target is now removed from `world.creatures`.
        let pkts = world
            .pending_outgoing
            .get(&conn)
            .expect("must have outgoing packets for the attacker");
        assert!(
            pkts.iter().any(|b| !b.is_empty() && b[0] == 0x84),
            "killing blow must send animated damage text (0x84) even after target death"
        );
    }

    /// Chargeable weapon at count=1 is destroyed on wearout (not stuck at 1).
    #[test]
    fn chargeable_weapon_wearout_destroys_last_charge() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        player.skills.fist = 50;
        player.sim_melee_attack = 20;
        let pid = world.creatures.insert(CreatureKind::Player(player));
        let mut cfg = MonsterAiConfig::default();
        cfg.defense = 0;
        cfg.armor = 0;
        let target = insert_monster_with_config(&mut world, "Rat", adjacent_pos(pos), 100, cfg);

        // Chargeable sword: ItemType.charges > 0, item.count = 1.
        const SWORD_ID: u16 = 2376;
        let it = ItemType {
            server_id: SWORD_ID,
            weapon_type: WEAPON_SWORD,
            attack: 20,
            charges: 100,
            ..Default::default()
        };
        if !world.items_db.items.contains_key(&SWORD_ID) {
            let mut items = std::collections::HashMap::clone(&world.items_db.items);
            items.insert(SWORD_ID, it);
            let client_to_server =
                std::collections::HashMap::clone(&world.items_db.client_to_server);
            world.items_db = std::sync::Arc::new(tfs_rust_content::items::ItemDatabase {
                items,
                client_to_server,
            });
        }
        let sword_iid = world.items.insert(Item::new(SWORD_ID, 1));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(pid) {
            let idx = crate::inventory::slot_to_array_index(InventorySlot::Left as u8).unwrap();
            p.equipment_slots[idx] = Some(sword_iid);
        }

        world.server_ms = 1000;
        world.player_close_attack_strike(pid, target);

        assert!(
            world.items.get(sword_iid).is_none(),
            "last charge must destroy the weapon"
        );
        assert!(
            world
                .get_player_inventory_item(pid, InventorySlot::Left as u8)
                .is_none(),
            "left hand slot must be cleared after wearout destroy"
        );
    }

    /// ProbeValue Increase before Get — a probe that levels the skill rolls with the new value
    /// (`crskill.cc:535-544`, audit B5).
    #[test]
    fn probe_value_rolls_with_post_increase_skill() {
        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let mut player = sim_hero_player("Hero", pos);
        // Sword skill 10, tries just below L11 need (None voc mult 2.0 → need 50).
        player.skills.sword = 10;
        player.skills.sword_tries = 49;
        player.base.learning_points = 30;
        player.sim_melee_attack = 0;
        let pid = world.creatures.insert(CreatureKind::Player(player));
        equip_item_sword(&mut world, pid, 2377, 20);
        let mut cfg = MonsterAiConfig::default();
        cfg.defense = 0;
        cfg.armor = 0;
        let target = insert_monster_with_config(&mut world, "Rat", adjacent_pos(pos), 500, cfg);

        world.server_ms = 1000;
        world.player_close_attack_strike(pid, target);

        let p = match world.creatures.get(pid) {
            Some(CreatureKind::Player(p)) => p,
            _ => panic!(),
        };
        assert_eq!(
            p.skills.sword, 11,
            "Increase(1) must level the skill before the damage ProbeValue Get()"
        );
        // DamageDone > 0 re-arms ActivateLearning → LearningPoints = 30 (`crcombat.cc:655`).
        assert_eq!(p.base.learning_points, 30);
    }

    fn equip_item_sword(world: &mut GameWorld, cid: CreatureId, item_type: u16, attack: i32) {
        let it = ItemType {
            server_id: item_type,
            weapon_type: WEAPON_SWORD,
            attack,
            defense: 10,
            ..Default::default()
        };
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
            let idx = crate::inventory::slot_to_array_index(InventorySlot::Left as u8).unwrap();
            p.equipment_slots[idx] = Some(iid);
        }
    }
}
