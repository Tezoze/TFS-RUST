//! Death: loot, XP from damage map, events, corpse decay placeholder.
//! C++ reference: `Creature::dropCorpse`, `Game::playerDeath`, `combat.cpp`;
//! 772 player death — `crmain.cc:790+` (AoL), `crplayer.cc:324` `TPlayer::Death` (skill/exp loss).

use crate::combat::distribute_experience;
use crate::config::ConfigManager;
use crate::creature::CreatureKind;
use crate::decay::DecayManager;
use crate::event_dispatcher::EventDispatcher;
use crate::formulas::StepSpeedModel;
use crate::ids::{CreatureId, ItemId};
use crate::item::Item;
use crate::party::split_shared_experience;
use slotmap::SlotMap;

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
// C++ reference: `Creature::onDeath` chain; monster XP — `crcombat.cc:891-908`.
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
        .any(|id| matches!(creatures.get(*id), Some(CreatureKind::Player(_))));

    let mut leveled_killers: Vec<CreatureId> = Vec::new();
    let mut xp_grants: Vec<XpShareGrant> = Vec::new();

    // Player victim: experience loss with bless reduction (PC-5 M7). Skill tries are
    // applied earlier via `GameWorld::apply_player_death_penalties`.
    if let Some(CreatureKind::Player(v)) = creatures.get_mut(victim) {
        let frac = death_loss_fraction(config, v.level, v.experience, v.blessings).clamp(0.0, 1.0);
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
        Some(CreatureKind::Player(p)) => (p.level.max(1) as u64).saturating_mul(100),
        _ => 0,
    };

    let mut killer_entries: Vec<(CreatureId, u64)> =
        damage_map.iter().map(|(&id, &dmg)| (id, dmg)).collect();
    killer_entries.sort_by_key(|(id, _)| *id);

    let shares: Vec<u64> = killer_entries.iter().map(|(_, dmg)| *dmg).collect();
    let grants = distribute_experience(exp_reward, &shares);

    for ((killer_id, _), share) in killer_entries.iter().zip(grants) {
        let share = if let Some(n) = party_size_for_xp.filter(|&n| n > 1) {
            split_shared_experience(share, n)
        } else {
            share
        };
        if let Some(CreatureKind::Player(k)) = creatures.get_mut(*killer_id) {
            let rate_exp = config
                .experience_rate_for_level(k.level)
                .unwrap_or(1.0)
                .max(0.0);
            let share = ((share as f64) * rate_exp).floor() as u64;
            if share > 0 {
                let old_level = k.level;
                if k.add_experience(share, step_speed_model) {
                    leveled_killers.push(*killer_id);
                }
                xp_grants.push(XpShareGrant {
                    cid: *killer_id,
                    amount: share,
                    old_level,
                    new_level: k.level,
                });
            }
        }
        events.on_kill(*killer_id, victim);
    }

    events.on_death(victim);

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

    #[test]
    fn zero_exp_loss_still_queues_victim_stats_grant() {
        // `deathLosePercent = 0` → lose == 0, but blessings still clear; must still emit a
        // victim grant so `apply_creature_death` calls `send_player_stats`.
        use crate::event_dispatcher::NullEventDispatcher;
        use crate::formulas::StepSpeedModel;
        use crate::sim_harness::{insert_player, minimal_world, test_player};
        use tfs_rust_common::Position;

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
        std::fs::write(
            &path,
            "depotFreeLimit = 2000\ndeathLosePercent = 0\n",
        )
        .expect("write config");
        world.config = std::rc::Rc::new(ConfigManager::load(&path).expect("load config"));

        let cid = insert_player(
            &mut world,
            {
                let mut p = test_player("ZeroLoss", Position::new(100, 100, 7));
                p.blessings = 0b11111;
                p.experience = 10_000;
                p.level = 20;
                p
            },
        );

        let (_, grants) = handle_creature_death(
            &mut world.creatures,
            &mut world.items,
            &mut world.decay,
            &NullEventDispatcher,
            cid,
            0,
            None,
            StepSpeedModel::LinearGo,
            world.config.as_ref(),
            false,
            0,
        );

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
}
