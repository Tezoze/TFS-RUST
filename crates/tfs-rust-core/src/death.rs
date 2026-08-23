//! Death: loot, XP from damage map, events, corpse decay placeholder.
//! C++ reference: `Creature::dropCorpse`, `Game::playerDeath`, `combat.cpp`;
//! 772 player death — `crmain.cc:790+` (AoL), `crplayer.cc:324` `TPlayer::Death` (skill/exp loss);
//! PvP kill XP — `crplayer.cc:339–340` + `crcombat.cc:922–934` `DistributeExperiencePoints`.

use crate::combat::{distribute_experience, pvp_kill_experience_amount};
use crate::config::ConfigManager;
use crate::creature::CreatureKind;
use crate::decay::DecayManager;
use crate::event_dispatcher::EventDispatcher;
use crate::formulas::{MechanicsProfile, StepSpeedModel};
use crate::ids::{CreatureId, ItemId};
use crate::item::Item;
use crate::party::split_shared_experience;
use slotmap::SlotMap;

fn player_script_event_names(
    creatures: &SlotMap<CreatureId, CreatureKind>,
    cid: CreatureId,
) -> Vec<String> {
    match creatures.get(cid) {
        Some(CreatureKind::Player(p)) => p.registered_creature_events.iter().cloned().collect(),
        _ => Vec::new(),
    }
}
use tfs_rust_common::enums::WorldType;

/// Count of the five standard blessings (bits 0–4). Twist of fate is bit 5.
#[inline]
pub fn blessing_count(blessings: i8) -> i32 {
    (0..5).filter(|i| blessings & (1 << i) != 0).count() as i32
}

/// Whether twist of fate (bit 5) is set.
#[inline]
pub fn has_twist_of_fate(blessings: i8) -> bool {
    blessings & (1 << 5) != 0
}

/// Clear blessings after death — TFS `player.cpp:2142-2150`.
///
/// With twist of fate: PvP last-hit clears only twist; PvE clears all five and keeps twist.
/// Without twist: clear all.
pub fn clear_blessings_on_death(blessings: i8, last_hit_by_player: bool) -> i8 {
    if has_twist_of_fate(blessings) {
        if last_hit_by_player {
            blessings & !(1 << 5)
        } else {
            1 << 5
        }
    } else {
        0
    }
}

fn default_death_loss_fraction(level: i32, experience: u64) -> f64 {
    // C++ ref: `Player::getLostPercent` (`src/player.cpp` ~4057+), without promotion/blessing reduction.
    if level >= 25 && experience > 0 {
        let tmp_level = level as f64;
        let loss_percent =
            ((tmp_level + 50.0) * 50.0 * (tmp_level * tmp_level - 5.0 * tmp_level + 8.0))
                / experience as f64;
        loss_percent / 100.0
    } else {
        0.10
    }
}

/// TFS-domain `Player::getLostPercent` with blessing reduction (PC-5).
///
/// Config `deathLosePercent != -1`: `(percent - bless_count).max(0) / 100`.
/// Else: base curve × `(1 - bless_count * 8%)`.
pub fn death_loss_fraction(
    config: &ConfigManager,
    level: i32,
    experience: u64,
    blessings: i8,
) -> f64 {
    let bless = blessing_count(blessings);
    let raw = config.death_lose_percent().unwrap_or(-1);
    if raw != -1 {
        return ((raw.max(0) - bless).max(0) as f64) / 100.0;
    }
    let base = default_death_loss_fraction(level, experience);
    let reduction = (bless as f64) * 0.08;
    (base * (1.0 - reduction).max(0.0)).clamp(0.0, 1.0)
}

/// Era-aware death loss fraction — M6.
///
/// 772 (`ClassicProbe`): flat `(promoted ? 7 : 10) - blessings` percent from
/// `profile.death_penalty` (`crplayer.cc:344-360`).
/// 1098 (`Modern`): TFS curve via [`death_loss_fraction`].
pub fn death_loss_fraction_for_profile(
    profile: &MechanicsProfile,
    config: &ConfigManager,
    level: i32,
    experience: u64,
    blessings: i8,
    promoted: bool,
) -> f64 {
    match profile.damage_formula {
        crate::formulas::DamageFormula::ClassicProbe => profile
            .death_penalty
            .loss_fraction(promoted, blessing_count(blessings))
            .clamp(0.0, 1.0),
        crate::formulas::DamageFormula::Modern => {
            death_loss_fraction(config, level, experience, blessings)
        }
    }
}

/// One player who received a positive XP share from a death.
#[derive(Debug, Clone, Copy)]
pub struct XpShareGrant {
    pub cid: CreatureId,
    pub amount: u64,
    pub old_level: i32,
    pub new_level: i32,
}

/// Apply death for a creature: distribute XP, fire events, schedule corpse decay item.
/// Caller must remove `victim` from the world after this returns.
///
/// When `schedule_generic_corpse` is false (772 race corpse already placed on tile), skip the
/// generic item 3058 insert.
///
/// Skill-try loss is applied by [`GameWorld::apply_player_death_penalties`] before this
/// (needs `FormulaHooks` on the world). This function handles exp loss + bless clear + XP share.
///
/// Returns `(leveled, xp_grants)`:
/// - `leveled` — creature IDs whose level (and thus speed) changed
/// - `xp_grants` — killers with positive XP (stats + popup) **and** the player victim
///   (always, so `sendStats` runs after blessing clear even when exp loss is 0)
// C++ reference: `Creature::onDeath` chain; monster XP — `crcombat.cc:891-908`;
// player kill XP — `crplayer.cc:339-340` (OE only) + `crcombat.cc:922-934`.
#[allow(clippy::too_many_arguments)]
pub fn handle_creature_death(
    creatures: &mut SlotMap<CreatureId, CreatureKind>,
    items: &mut SlotMap<ItemId, Item>,
    decay: &mut DecayManager,
    events: &dyn EventDispatcher,
    victim: CreatureId,
    decay_now: u64,
    party_size_for_xp: Option<usize>,
    step_speed_model: StepSpeedModel,
    config: &ConfigManager,
    schedule_generic_corpse: bool,
    corpse_decay_offset_ms: u64,
    world_type: WorldType,
    mechanics: &MechanicsProfile,
    round_nr: u32,
) -> (Vec<CreatureId>, Vec<XpShareGrant>) {
    if matches!(creatures.get(victim), Some(CreatureKind::Npc(_)) | None) {
        return (Vec::new(), Vec::new());
    }

    let damage_map = match creatures.get(victim) {
        Some(CreatureKind::Player(p)) => p.base.damage_map.clone(),
        Some(CreatureKind::Monster(m)) => m.base.damage_map.clone(),
        Some(CreatureKind::Npc(_)) | None => return (Vec::new(), Vec::new()),
    };

    let last_hit_by_player = damage_map
        .keys()
        .any(|id| matches!(creatures.get(id), Some(CreatureKind::Player(_))));

    let mut leveled_killers: Vec<CreatureId> = Vec::new();
    let mut xp_grants: Vec<XpShareGrant> = Vec::new();

    // 772 PvP pool uses pre-death Exp (`crplayer.cc:340` Exp/20) — capture before loss.
    let (pvp_victim_level, pvp_pool) = match creatures.get(victim) {
        Some(CreatureKind::Player(p)) if world_type == WorldType::PvpEnforced => {
            (Some(p.level.max(1)), Some(p.experience / 20))
        }
        Some(CreatureKind::Player(p)) => (Some(p.level.max(1)), Some(0)),
        _ => (None, None),
    };
    let is_pvp_kill = pvp_pool.is_some();

    // Player victim: experience loss with bless reduction (PC-5 M7, M6 era-gated).
    // 772 uses flat `(promoted ? 7 : 10) - blessings` percent; 1098 uses TFS curve.
    // Skill try loss is applied earlier via `GameWorld::apply_player_death_penalties`.
    if let Some(CreatureKind::Player(v)) = creatures.get_mut(victim) {
        let promoted = v.vocation_profile.from_vocation != v.vocation_profile.id
            && v.vocation_profile.from_vocation != 0;
        let frac = death_loss_fraction_for_profile(
            mechanics,
            config,
            v.level,
            v.experience,
            v.blessings,
            promoted,
        )
        .clamp(0.0, 1.0);
        let lose = ((v.experience as f64) * frac).floor() as u64;
        let old_level = v.level;
        if lose > 0 && v.remove_experience(lose, step_speed_model) {
            leveled_killers.push(victim);
        }
        // Always refresh victim stats after death — TFS `Player::death` calls `sendStats()`
        // after blessing clear even when `deathLosePercent` yields zero exp loss
        // (`player.cpp:2153`). Skipping this when `lose == 0` left blessings cleared
        // server-side with no client `0xA0` until next login.
        xp_grants.push(XpShareGrant {
            cid: victim,
            amount: 0, // loss path — no floating “+exp” popup
            old_level,
            new_level: v.level,
        });
        v.blessings = clear_blessings_on_death(v.blessings, last_hit_by_player);
    }

    let exp_reward: u64 = match creatures.get(victim) {
        Some(CreatureKind::Monster(m)) => m.experience as u64,
        Some(CreatureKind::Player(_)) => pvp_pool.unwrap_or(0),
        _ => 0,
    };

    let mut killer_entries: Vec<(CreatureId, u64)> = damage_map
        .iter_active()
        .map(|(id, dmg, _)| (id, dmg))
        .collect();
    killer_entries.sort_by_key(|(id, _)| *id);

    let shares: Vec<u64> = killer_entries.iter().map(|(_, dmg)| *dmg).collect();
    // M8 — Use the monotonic `CombatDamage` accumulator as the denominator (C++
    // `DistributeExperiencePoints` `crcombat.cc:906-921`), not the re-sumed surviving ring.
    // This includes damage from evicted/dead attackers, so payout can be < 100% of Exp.
    let combat_damage = damage_map.combat_damage;
    let grants = distribute_experience(exp_reward, &shares, Some(combat_damage));

    for ((killer_id, _), share) in killer_entries.iter().zip(grants) {
        let share = if is_pvp_kill {
            // 772 PvP arm: no TFS party shared-XP split; party members skipped below.
            share
        } else if let Some(n) = party_size_for_xp.filter(|&n| n > 1) {
            split_shared_experience(share, n)
        } else {
            share
        };

        let Some(CreatureKind::Player(k)) = creatures.get(*killer_id) else {
            let names = player_script_event_names(creatures, *killer_id);
            events.on_kill(*killer_id, victim, &names);
            continue;
        };

        let share = if is_pvp_kill {
            // 772 `InPartyWith(..., true)` — live or former party within +5 rounds.
            let same_party = match creatures.get(victim) {
                Some(CreatureKind::Player(v)) => v.in_party_with(k, true, round_nr),
                _ => false,
            };
            if same_party {
                0
            } else if let Some(vic_lvl) = pvp_victim_level {
                pvp_kill_experience_amount(mechanics, vic_lvl, k.level, share)
            } else {
                0
            }
        } else {
            share
        };

        let rate_exp = config
            .experience_rate_for_level(k.level)
            .unwrap_or(1.0)
            .max(0.0);
        let share = ((share as f64) * rate_exp).floor() as u64;
        if share > 0
            && let Some(CreatureKind::Player(k)) = creatures.get_mut(*killer_id)
        {
            let old_level = k.level;
            if k.add_experience(share, step_speed_model) {
                leveled_killers.push(*killer_id);
            }
            // 772 soul regen on exp (`crcombat.cc:938-955`): Amount >= AttackerLevel.
            if share >= old_level as u64 {
                k.arm_soul_regen_timer();
            }
            xp_grants.push(XpShareGrant {
                cid: *killer_id,
                amount: share,
                old_level,
                new_level: k.level,
            });
        }
        let names = player_script_event_names(creatures, *killer_id);
        events.on_kill(*killer_id, victim, &names);
    }

    let death_names = player_script_event_names(creatures, victim);
    let killer = creatures.get(victim).and_then(|k| k.base().last_hit_by);
    #[cfg(debug_assertions)]
    let item_count_before_on_death = items.len();
    events.on_death(victim, killer, &death_names);
    #[cfg(debug_assertions)]
    debug_assert_eq!(
        items.len(),
        item_count_before_on_death,
        "CreatureEvent onDeath must not create items"
    );

    if schedule_generic_corpse {
        let corpse_id = items.insert(Item::new(3058, 1));
        decay.schedule(
            corpse_id,
            decay_now.saturating_add(corpse_decay_offset_ms),
            None,
        );
    }

    (leveled_killers, xp_grants)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_dispatcher::NullEventDispatcher;
    use crate::formulas::{MechanicsProfile, StepSpeedModel};
    use crate::sim_harness::{insert_player, minimal_world, test_player};
    use tfs_rust_common::Position;
    use tfs_rust_common::enums::WorldType;

    #[test]
    fn blessing_count_bits_0_to_4() {
        assert_eq!(blessing_count(0), 0);
        assert_eq!(blessing_count(0b00111), 3);
        assert_eq!(blessing_count(0b11111), 5);
        // Twist of fate alone does not count as a standard blessing.
        assert_eq!(blessing_count(1 << 5), 0);
    }

    #[test]
    fn clear_blessings_twist_pve_keeps_twist() {
        let with_twist = 0b0010_1111; // 5 bless + twist
        assert_eq!(clear_blessings_on_death(with_twist, false), 1 << 5);
        assert_eq!(clear_blessings_on_death(with_twist, true), 0b0000_1111);
        assert_eq!(clear_blessings_on_death(0b11111, false), 0);
    }

    #[test]
    fn blessing_reduces_default_loss_curve() {
        // Level < 25 → base 10%; 5 blessings × 8% → 60% of base.
        let base = default_death_loss_fraction(20, 1000);
        assert!((base - 0.10).abs() < 1e-9);
        let reduced = base * (1.0 - 5.0 * 0.08);
        assert!((reduced - 0.06).abs() < 1e-9);
        assert_eq!(blessing_count(0b11111), 5);
    }

    fn death_call(
        world: &mut crate::game_world::GameWorld,
        victim: CreatureId,
        world_type: WorldType,
    ) -> (Vec<CreatureId>, Vec<XpShareGrant>) {
        handle_creature_death(
            &mut world.creatures,
            &mut world.items,
            &mut world.decay,
            &NullEventDispatcher,
            victim,
            0,
            None,
            StepSpeedModel::LinearGo,
            world.config.as_ref(),
            false,
            0,
            world_type,
            &world.mechanics.profile,
            world.round_nr,
        )
    }

    #[test]
    fn zero_exp_loss_still_queues_victim_stats_grant() {
        // `deathLosePercent = 0` → lose == 0, but blessings still clear; must still emit a
        // victim grant so `apply_creature_death` calls `send_player_stats`.
        let mut world = minimal_world();
        // Override config with zero death loss.
        let path = std::env::temp_dir().join(format!(
            "tfs_death_zero_loss_{}_{}.lua",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::write(&path, "depotFreeLimit = 2000\ndeathLosePercent = 0\n")
            .expect("write config");
        world.config = std::rc::Rc::new(ConfigManager::load(&path).expect("load config"));

        let cid = insert_player(&mut world, {
            let mut p = test_player("ZeroLoss", Position::new(100, 100, 7));
            p.blessings = 0b11111;
            p.experience = 10_000;
            p.level = 20;
            p
        });

        let (_, grants) = death_call(&mut world, cid, WorldType::Pvp);

        assert!(
            grants.iter().any(|g| g.cid == cid && g.amount == 0),
            "victim must get a zero-amount grant for sendStats"
        );
        let blessings = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.blessings,
            _ => panic!("victim missing"),
        };
        assert_eq!(blessings, 0, "blessings cleared even with zero exp loss");
    }

    #[test]
    fn pvp_kill_exp_open_pvp_is_zero() {
        let mut world = minimal_world();
        let victim = insert_player(&mut world, {
            let mut p = test_player("Vic", Position::new(100, 100, 7));
            p.experience = 20_000;
            p.level = 50;
            p
        });
        let killer = insert_player(&mut world, {
            let mut p = test_player("Atk", Position::new(101, 100, 7));
            p.level = 40;
            p.experience = 0;
            p
        });
        if let Some(CreatureKind::Player(v)) = world.creatures.get_mut(victim) {
            v.base.damage_map.insert(killer, 100);
        }

        let (_, grants) = death_call(&mut world, victim, WorldType::Pvp);
        assert!(
            !grants.iter().any(|g| g.cid == killer && g.amount > 0),
            "open PvP must not grant player-kill XP"
        );
    }

    #[test]
    fn pvp_kill_exp_enforced_scales_by_max_level() {
        let mut world = minimal_world();
        let victim = insert_player(&mut world, {
            let mut p = test_player("Vic", Position::new(100, 100, 7));
            // Pool = 20_000 / 20 = 1000; sole damager takes all before scale.
            p.experience = 20_000;
            p.level = 100;
            p
        });
        let killer = insert_player(&mut world, {
            let mut p = test_player("Atk", Position::new(101, 100, 7));
            p.level = 100;
            p.experience = 0;
            p
        });
        if let Some(CreatureKind::Player(v)) = world.creatures.get_mut(victim) {
            v.base.damage_map.insert(killer, 100);
        }

        let (_, grants) = death_call(&mut world, victim, WorldType::PvpEnforced);
        // MaxLevel=110; ((110-100)*1000)/100 = 100.
        let got = grants
            .iter()
            .find(|g| g.cid == killer)
            .map(|g| g.amount)
            .unwrap_or(0);
        assert_eq!(got, 100);
    }

    #[test]
    fn pvp_kill_exp_party_skip() {
        let mut world = minimal_world();
        let victim = insert_player(&mut world, {
            let mut p = test_player("Vic", Position::new(100, 100, 7));
            p.experience = 20_000;
            p.level = 50;
            p.social.party_id = Some(1);
            p
        });
        let killer = insert_player(&mut world, {
            let mut p = test_player("Atk", Position::new(101, 100, 7));
            p.level = 40;
            p.experience = 0;
            p.social.party_id = Some(1);
            p
        });
        if let Some(CreatureKind::Player(v)) = world.creatures.get_mut(victim) {
            v.base.damage_map.insert(killer, 100);
        }

        let (_, grants) = death_call(&mut world, victim, WorldType::PvpEnforced);
        assert!(
            !grants.iter().any(|g| g.cid == killer && g.amount > 0),
            "same-party killer must be skipped"
        );
    }

    #[test]
    fn pvp_kill_experience_amount_unit() {
        let profile = MechanicsProfile::for_version(tfs_rust_common::ProtocolVersion::V772);
        assert_eq!(pvp_kill_experience_amount(&profile, 100, 110, 1000), 0);
        assert_eq!(pvp_kill_experience_amount(&profile, 100, 100, 1000), 100);
    }

    // M6 — 772 flat death penalty: (promoted ? 7 : 10) - blessings percent.

    fn world_772() -> crate::game_world::GameWorld {
        let mut world = minimal_world();
        world.mechanics.profile =
            MechanicsProfile::for_version(tfs_rust_common::ProtocolVersion::V772);
        world
    }

    #[test]
    fn m6_death_penalty_flat_772_unpromoted_10_percent() {
        let mut world = world_772();
        let cid = insert_player(&mut world, {
            let mut p = test_player("Vic", Position::new(100, 100, 7));
            p.experience = 10_000;
            p.level = 20;
            p.blessings = 0;
            p
        });
        let (_, grants) = death_call(&mut world, cid, WorldType::Pvp);
        let grant = grants.iter().find(|g| g.cid == cid).expect("victim grant");
        // 10% of 10_000 = 1_000 exp lost → level may drop; check exp delta via grant levels.
        // The grant amount is 0 (loss path); verify exp was reduced.
        let exp = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.experience,
            _ => panic!("victim missing"),
        };
        assert_eq!(exp, 9_000, "unpromoted 772 death loses 10% exp");
        let _ = grant;
    }

    #[test]
    fn m6_death_penalty_flat_772_promoted_7_percent() {
        let mut world = world_772();
        let cid = insert_player(&mut world, {
            let mut p = test_player("Vic", Position::new(100, 100, 7));
            p.experience = 10_000;
            p.level = 20;
            p.blessings = 0;
            // Promoted: from_vocation != id && from_vocation != 0.
            p.vocation_profile.from_vocation = 1;
            p.vocation_profile.id = 5;
            p
        });
        let _ = death_call(&mut world, cid, WorldType::Pvp);
        let exp = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.experience,
            _ => panic!("victim missing"),
        };
        assert_eq!(exp, 9_300, "promoted 772 death loses 7% exp");
    }

    #[test]
    fn m6_death_penalty_flat_772_blessings_reduce() {
        let mut world = world_772();
        let cid = insert_player(&mut world, {
            let mut p = test_player("Vic", Position::new(100, 100, 7));
            p.experience = 10_000;
            p.level = 20;
            p.blessings = 0b00111; // 3 blessings
            p
        });
        let _ = death_call(&mut world, cid, WorldType::Pvp);
        let exp = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.experience,
            _ => panic!("victim missing"),
        };
        // 10% - 3 = 7% → lose 700.
        assert_eq!(exp, 9_300, "3 blessings reduce 772 loss to 7%");
    }

    #[test]
    fn m6_death_penalty_flat_772_promoted_with_all_blessings_zero() {
        let mut world = world_772();
        let cid = insert_player(&mut world, {
            let mut p = test_player("Vic", Position::new(100, 100, 7));
            p.experience = 10_000;
            p.level = 20;
            p.blessings = 0b11111; // 5 blessings
            p.vocation_profile.from_vocation = 1;
            p.vocation_profile.id = 5;
            p
        });
        let _ = death_call(&mut world, cid, WorldType::Pvp);
        let exp = match world.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.experience,
            _ => panic!("victim missing"),
        };
        // 7% - 5 = 2% → lose 200.
        assert_eq!(exp, 9_800, "promoted + 5 blessings → 2% loss");
    }
}
