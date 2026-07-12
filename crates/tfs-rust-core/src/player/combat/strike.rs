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
//! Reuses `combat::math::{weapon_damage, armor_reduction, melee_damage_after_defense_and_armor}`,
//! `roll_target_defense`, and `combat_execute_with_stimulus` — no parallel player combat math
//! module (`tfs-code-hygiene.md`). Era knobs flow through `MechanicsProfile`/`FormulaHooks`;
//! per-vocation `formula.melee_damage` from the cached `VocationProfile` snapshot.

use rand::rngs::StdRng;
use rand::SeedableRng;

use tfs_rust_common::enums::CombatType;

use crate::combat::math::{armor_reduction, melee_damage_after_defense_and_armor, weapon_damage};
use crate::combat::{CombatDamage, CombatParams};
use crate::creature::{roll_target_defense, CreatureKind};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

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
        let hooks = &self.mechanics.hooks;

        // `GetAttackValue` (PC-1) + attacker skill/level/mode/vocation block — read before
        // any mutation so the math holds no `&mut` borrows.
        let (atk_value, atk_skill_nr) = self.player_get_attack_value(cid);
        let (skill, level, mode, melee_mult, attack_speed_ms, learning_active) =
            match self.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => (
                    atk_skill_nr.level(&p.skills),
                    p.level,
                    p.attack_mode,
                    p.vocation_profile.formula.melee_damage as f64,
                    p.vocation_profile.attack_speed_ms as u64,
                    p.base.learning_points > 0,
                ),
                _ => return,
            };

        // Target defense/armor snapshot — world-aware so a player target contributes
        // shield/weapon defend + shielding skill + armor (`melee_defense_snapshot_for`).
        let defense_snap = self.melee_defense_snapshot_for(target_id);

        // `DelayAttack(200)` before the strike (`crcombat.cc:608`).
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().delay_attack_ms(server_ms, 200);
        }

        let mut rng = std::mem::replace(&mut self.ai_rng, StdRng::from_entropy());

        // `GetAttackDamage` — fight-mode-scaled probe roll (`crcombat.cc:220`).
        let attack_roll = weapon_damage(&profile, hooks, &mut rng, skill, atk_value, mode, level);
        // Vocation `formula.melee_damage` multiplier (PC-2 step 1).
        let attack_roll = ((attack_roll as f64) * melee_mult).floor() as i32;

        // `ProbeValue` side-effect: decrement `LearningPoints` while > 0 (`crskill.cc:549`).
        // `Increase(1)` skill-exp is PC-5 (§0.5 — no tries counters yet).
        if learning_active {
            if let Some(k) = self.creatures.get_mut(cid) {
                let lp = &mut k.base_mut().learning_points;
                if *lp > 0 {
                    *lp -= 1;
                }
            }
        }

        // `target.GetDefendDamage` — gate + probe (`crcombat.cc:236`).
        // M11 — shield wearout happens after the defense probe, only when the gate passes
        // (`crcombat.cc:265-281`). Capture the gate state before the probe so we know whether
        // `roll_target_defense` actually ran the defense (gate pass → timestamps updated).
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

        // Armor mitigation — applied inside `Damage(PHYSICAL)` in C++ (`crcombat.cc:302`);
        // here it feeds `melee_damage_after_defense_and_armor` so the shared physical path
        // (`combat_execute_with_stimulus`) receives the post-armor HP delta.
        let armor_roll = armor_reduction(&profile, hooks, &mut rng, defense_snap.armor);
        // M11 — Shield wearout: decrement the defender's shield `REMAININGUSES` when the defense
        // gate passed and the defender has a chargeable shield equipped (`crcombat.cc:265-281`).
        // Player-only (monsters don't have shields). Called after `hooks` is last used to avoid
        // borrow conflict with `&mut self`.
        if defense_gate_passed {
            self.player_shield_wearout(target_id);
        }
        let dmg = melee_damage_after_defense_and_armor(attack_roll, defense_roll, armor_roll);

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
        let _ = self.combat_execute_with_stimulus(
            Some(cid),
            target_id,
            &CombatDamage {
                primary: (CombatType::Physical, -dmg),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
        );
        // `combat_execute_with_stimulus` calls `apply_creature_death` when HP ≤ 0, which
        // removes the target from `world.creatures`. So a missing key means the target died
        // on this strike — `hp_after = 0` so `damage_done` reflects the killing blow and the
        // `StopAttack` branch below fires (`crcombat.cc:656`).
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
            self.notify_player_combat_damage(Some(cid), target_id, damage_done, snap);
        }
        self.ai_rng = rng;

        // `if (DamageDone > 0) ActivateLearning()` (`crcombat.cc:655`).
        if damage_done > 0 {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().activate_learning();
            }
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
    /// weapons/shields model remaining uses via `count`). No-op for standard weapons
    /// (`charges == 0`). Full wearout — initializing `count = charges` at equip time and
    /// unequipping/destroying on `count == 0` — is a follow-up; PC-2 wires the per-strike
    /// decrement hook so chargeable items tick down. Sends `sendInventoryItem` (0x78) so the
    /// client refreshes the charge count immediately.
    fn player_strike_weapon_wearout(&mut self, cid: CreatureId) {
        let weapon_iid = match self.player_get_weapon(cid, true) {
            Some(iid) => iid,
            None => return,
        };
        // Read the item type's `charges` flag without holding borrows across the mutation.
        let has_charges = {
            let Some(item) = self.items.get(weapon_iid) else {
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
        let mut decremented = false;
        if let Some(item) = self.items.get_mut(weapon_iid) {
            if item.count > 1 {
                item.count -= 1;
                decremented = true;
            }
        }
        if decremented {
            // Resolve the hand slot holding the weapon so the client refreshes the charge count.
            let slot = self.equipment_slot_for_item(cid, weapon_iid);
            if let Some(slot) = slot {
                self.broadcast_player_inventory_slot(cid, slot, Some(weapon_iid));
            }
        }
    }

    /// M11 — Shield wearout for the defender — `crcombat.cc:265-281` `RemainingUses--`.
    ///
    /// Decrements the defender's shield `count` when `ItemType.charges > 0` (chargeable
    /// shields model remaining uses via `count`). No-op for standard shields (`charges == 0`)
    /// or when no shield is equipped. Player-only — monsters don't have shields. Called after
    /// `roll_target_defense` when the defense gate passed (the C++ `GetDefendDamage` decrements
    /// after the probe, inside the gate-passed block). Sends `sendInventoryItem` (0x78) so the
    /// client refreshes the charge count immediately.
    pub(crate) fn player_shield_wearout(&mut self, cid: CreatureId) {
        let shield_iid = match self.player_get_shield(cid) {
            Some(iid) => iid,
            None => return,
        };
        let has_charges = {
            let Some(item) = self.items.get(shield_iid) else {
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
        let mut decremented = false;
        if let Some(item) = self.items.get_mut(shield_iid) {
            if item.count > 1 {
                item.count -= 1;
                decremented = true;
            }
        }
        if decremented {
            let slot = self.equipment_slot_for_item(cid, shield_iid);
            if let Some(slot) = slot {
                self.broadcast_player_inventory_slot(cid, slot, Some(shield_iid));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::{CreatureKind, MonsterAiConfig};
    use crate::inventory::{InventorySlot, WEAPON_SHIELD};
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
        let mut world = minimal_world();
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
}
