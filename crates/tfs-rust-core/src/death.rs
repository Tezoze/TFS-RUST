//! Death: loot, XP from damage map, events, corpse decay placeholder.
// C++ reference: `Creature::dropCorpse`, `Game::playerDeath`, `combat.cpp`.

use crate::creature::CreatureKind;
use crate::decay::DecayManager;
use crate::event_dispatcher::EventDispatcher;
use crate::ids::{CreatureId, ItemId};
use crate::item::Item;
use crate::party::split_shared_experience;
use slotmap::SlotMap;
/// Apply death for a creature: distribute XP, fire events, schedule corpse decay item.
/// Caller must remove `victim` from the world after this returns.
// C++ reference: `Creature::onDeath` chain.
pub fn handle_creature_death(
    creatures: &mut SlotMap<CreatureId, CreatureKind>,
    items: &mut SlotMap<ItemId, Item>,
    decay: &mut DecayManager,
    events: &dyn EventDispatcher,
    victim: CreatureId,
    tick: u64,
    party_size_for_xp: Option<usize>,
) {
    if matches!(creatures.get(victim), Some(CreatureKind::Npc(_)) | None) {
        return;
    }

    let damage_map = match creatures.get(victim) {
        Some(CreatureKind::Player(p)) => p.base.damage_map.clone(),
        Some(CreatureKind::Monster(m)) => m.base.damage_map.clone(),
        Some(CreatureKind::Npc(_)) | None => return,
    };

    let exp_reward: u64 = match creatures.get(victim) {
        Some(CreatureKind::Monster(m)) => (m.base.max_health.max(1) as u64).saturating_mul(4),
        Some(CreatureKind::Player(p)) => (p.level.max(1) as u64).saturating_mul(100),
        _ => 0,
    };

    let total_damage: u64 = damage_map.values().sum();

    for (&killer_id, &dmg) in &damage_map {
        if total_damage == 0 {
            break;
        }
        let share = exp_reward.saturating_mul(dmg) / total_damage;
        let share = if let Some(n) = party_size_for_xp.filter(|&n| n > 1) {
            split_shared_experience(share, n)
        } else {
            share
        };
        if let Some(CreatureKind::Player(k)) = creatures.get_mut(killer_id) {
            k.add_experience(share);
        }
        events.on_kill(killer_id, victim);
    }

    events.on_death(victim);

    let corpse_id = items.insert(Item::new(ItemId::default(), 3058, 1));
    decay.schedule(corpse_id, tick.saturating_add(600), None);
}
