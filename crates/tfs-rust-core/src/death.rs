//! Death: loot, XP from damage map, events, corpse decay placeholder.
// C++ reference: `Creature::dropCorpse`, `Game::playerDeath`, `combat.cpp`.

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

fn death_loss_fraction(config: &ConfigManager, level: i32, experience: u64) -> f64 {
    let raw = config.death_lose_percent().unwrap_or(-1);
    if raw != -1 {
        return (raw.max(0) as f64) / 100.0;
    }
    default_death_loss_fraction(level, experience)
}
/// Apply death for a creature: distribute XP, fire events, schedule corpse decay item.
/// Caller must remove `victim` from the world after this returns.
///
/// When `schedule_generic_corpse` is false (772 race corpse already placed on tile), skip the
/// generic item 3058 insert.
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
) -> Vec<CreatureId> {
    if matches!(creatures.get(victim), Some(CreatureKind::Npc(_)) | None) {
        return Vec::new();
    }

    let damage_map = match creatures.get(victim) {
        Some(CreatureKind::Player(p)) => p.base.damage_map.clone(),
        Some(CreatureKind::Monster(m)) => m.base.damage_map.clone(),
        Some(CreatureKind::Npc(_)) | None => return Vec::new(),
    };

    // Apply victim death loss (separate from gain rates / stages).
    let mut leveled_killers: Vec<CreatureId> = Vec::new();

    if let Some(CreatureKind::Player(v)) = creatures.get_mut(victim) {
        let frac = death_loss_fraction(config, v.level, v.experience).clamp(0.0, 1.0);
        let lose = ((v.experience as f64) * frac).floor() as u64;
        if lose > 0 && v.remove_experience(lose, step_speed_model) {
            leveled_killers.push(victim);
        }
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
            if k.add_experience(share, step_speed_model) {
                leveled_killers.push(*killer_id);
            }
        }
        events.on_kill(*killer_id, victim);
    }

    events.on_death(victim);

    if schedule_generic_corpse {
        let corpse_id = items.insert(Item::new(3058, 1));
        // K2: era-tuned corpse decay offset (772 30 000 ms, 1098 600 ms) from MechanicsProfile.
        decay.schedule(corpse_id, decay_now.saturating_add(corpse_decay_offset_ms), None);
    }

    // Return killers whose level (and thus speed) changed so the caller can
    // `announce_creature_speed` — C++ `cract.cc:1637` `CREATURE_SPEED_CHANGED`.
    leveled_killers
}
