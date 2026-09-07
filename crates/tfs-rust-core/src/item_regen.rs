//! Item regen on the Creatures arm — equipped `SkillNumber=14` DAct cadence.
//!
//! C++ reference: `crmain.cc:1087-1095` `ProcessCreatures` item regen,
//! `crskill.cc:19-23` `TSkill::Get` = Act + DAct, `cract.cc:1639-1660`
//! `NotifyChangeInventory` SkillNumber 14. Eating never writes Act
//! (`moveuse.cc:1846` `SetTimer` Cycle only). Amounts are corpus constants
//! (+1 HP / +4 mana), not `items.xml` healthgain/managain.

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// Sum `health_ticks / 1000` over equipped regen items (`TSkill::Get` DAct).
///
/// Returns 0 when nothing equipped — no Creatures-arm item regen.
pub fn fed_regen_interval(world: &GameWorld, cid: CreatureId) -> u32 {
    let Some(CreatureKind::Player(p)) = world.creatures.get(cid) else {
        return 0;
    };
    // Store-inbox (index 10) is not an equipment slot for SkillNumber-14.
    let slots: [Option<crate::ids::ItemId>; 10] = std::array::from_fn(|i| p.equipment_slots[i]);
    let mut sum = 0u32;
    for iid in slots.into_iter().flatten() {
        let Some(item) = world.items.get(iid) else {
            continue;
        };
        let Some(it) = world.items_db.items.get(&item.item_type) else {
            continue;
        };
        if it.abilities.regeneration && it.abilities.health_ticks >= 1000 {
            sum = sum.saturating_add(it.abilities.health_ticks / 1000);
        }
    }
    sum
}

impl GameWorld {
    /// Recompute [`crate::creature::Player::item_regen_interval`] from current equipment.
    /// Call from the equip/de-equip hook after ability deltas.
    pub(crate) fn recompute_item_regen_interval(&mut self, cid: CreatureId) {
        let interval = fed_regen_interval(self, cid);
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.item_regen_interval = interval;
        }
    }

    /// 772 `ProcessCreatures` item regen (`crmain.cc:1087-1095`).
    ///
    /// Gate: `interval > 0 && RoundNr % interval == 0 && health > 0 && !PZ`.
    /// Returns `true` when HP or mana was granted.
    pub(crate) fn process_item_regen(&mut self, cid: CreatureId, round_nr: u32) -> bool {
        let (interval, pos, health) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => {
                (p.item_regen_interval, p.base.position, p.base.health)
            }
            _ => return false,
        };
        // `!IsDead` is health<=0 until M1 deferred death (`crmain.cc:1087-1095`).
        if interval == 0 || health <= 0 || !round_nr.is_multiple_of(interval) {
            return false;
        }
        if self.tile_in_protection_zone(pos) {
            return false;
        }

        let hp = self.mechanics.profile.item_regen_hp;
        let mana = self.mechanics.profile.item_regen_mana;
        let snap = self.combat_notify_snapshot(cid);
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.base.health = (p.base.health + hp).min(p.base.max_health);
            p.mana = (p.mana + mana).min(p.max_mana);
        }
        if let Some(snap) = snap {
            self.notify_creature_healed(cid, snap);
        } else {
            self.send_player_stats(cid);
        }
        true
    }
}
