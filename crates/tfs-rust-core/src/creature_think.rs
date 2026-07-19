//! Creature think cadence — 772 `ProcessCreatures` + ToDo/IdleStimulus engine.
//!
//! - 772 `ProcessCreatures` — `crmain.cc:1075–1138` (item regen + PK marks + death safety).
//! - Monster AI is driven by the ToDoQueue / `IdleStimulus` / `CreatureMoveStimulus` /
//!   `DamageStimulus`, not by a per-creature `onThink` sweep.
//!
//! The TFS 1098 `checkCreatures` bucket sweep, `Creature::onThink`, `Creature::onAttacking`,
//! `Monster::onThink`, and `Npc::onThink` were deleted in Phase 3–5 of the unified beat
//! engine effort. Both eras now run on the 772 ToDo engine. The accompanying
//! `addCreatureCheck` / `removeCreatureCheck` bucket accounting (`game.cpp` ~3798–3828)
//! was removed alongside — the bucket index was never incremented after the sweep
//! deletion, so the per-creature `think_check_bucket` field and helpers were dead.

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
#[allow(unused_imports)] // re-exported for `creature_think_tests` via `super::*`
use crate::ids::CreatureId;

/// TFS `creature.h` `EVENT_CREATURE_THINK_INTERVAL` — used by the ToDo attack cadence.
pub const EVENT_CREATURE_THINK_INTERVAL_MS: u32 = 1000;

impl GameWorld {
    /// 772 `ProcessCreatures` — item regen + PK-mark clearing + death safety (`crmain.cc:1075–1138`).
    ///
    /// **Not** an AI think sweep. C++ `ProcessCreatures` does:
    /// 1. **Item regen** (HP+1/Mana+4) gated on `SKILL_FED` `Get()` = `food_level`
    ///    (`crmain.cc:1087-1095`) — separate from vocation regen in `TSkillFed::Event`.
    /// 2. **PK-mark clearing** on `EarliestLogoutRound` expiry (`crmain.cc:1102-1105`).
    /// 3. **Death safety net** (`HP <= 0 && !IsDead → Death()`).
    ///
    /// It does **not** call `onThink`, `IdleStimulus`, target validation, or idle status updates —
    /// monster AI is driven entirely by the ToDoQueue / `IdleStimulus` / `CreatureMoveStimulus` /
    /// `DamageStimulus`.
    ///
    /// Vocation regen (HP/mana from `TSkillFed::Event`) is handled by `process_skills` →
    /// `process_player_fed_regen` (`process_skills.rs:29`). Logout is handled by
    /// `process_connections` / `pending_idle_kick`.
    pub fn process_creatures(&mut self) {
        // C++ iterates all creatures (`FirstFreeCreature`). Item regen + PK marks are
        // player-only; death safety is the only work that applies to monsters/NPCs. Split
        // the pass so ~20k sleeping spawn monsters are not re-touched for food/PK every
        // CreatureTimeCounter fire (same lag class as `process_skills`).
        let round_nr = self.round_nr;

        self.scratch_stats_dirty.clear();
        self.scratch_pk_marks.clear();
        self.scratch_dead.clear();

        for (cid, k) in self.creatures.iter() {
            if k.base().health <= 0 {
                self.scratch_dead.push(cid);
            }
            let CreatureKind::Player(p) = k else {
                continue;
            };
            // C++ `ProcessCreatures` item regen (`crmain.cc:1087-1095`):
            //   RegenInterval = Skills[SKILL_FED]->Get();  // food_level (Act)
            //   if(RegenInterval > 0 && (RoundNr % RegenInterval) == 0
            //      && !IsDead && !IsProtectionZone(pos))
            //       HP += 1; Mana += 4; SendPlayerData();
            if p.food_level > 0
                && p.base.health > 0
                && round_nr.is_multiple_of(p.food_level as u32)
                && !self.tile_in_protection_zone(p.base.position)
            {
                self.scratch_stats_dirty.push(cid);
            }
            // C++ PK-mark clearing (`crmain.cc:1102-1105`).
            if p.earliest_logout_round != 0 && p.earliest_logout_round <= round_nr {
                self.scratch_pk_marks.push(cid);
            }
        }

        for cid in std::mem::take(&mut self.scratch_stats_dirty) {
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.base.health = (p.base.health + 1).min(p.base.max_health);
                p.mana = (p.mana + 4).min(p.max_mana);
            }
            self.send_player_stats(cid);
        }

        for cid in std::mem::take(&mut self.scratch_pk_marks) {
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.earliest_logout_round = 0;
            }
            // Drop TFS-domain Infight when the 772 logout round expires (`CheckState` clear).
            let removed_infight = if let Some(kind) = self.creatures.get_mut(cid) {
                let before = kind.base().active_conditions.len();
                kind.base_mut()
                    .active_conditions
                    .retain(|c| c.ctype != tfs_rust_common::enums::ConditionType::Infight);
                before != kind.base().active_conditions.len()
            } else {
                false
            };
            if removed_infight {
                self.on_condition_ended(cid, tfs_rust_common::enums::ConditionType::Infight);
            } else if matches!(self.creatures.get(cid), Some(CreatureKind::Player(_))) {
                self.send_player_icons(cid);
            }
            tracing::debug!(
                ?cid,
                round_nr,
                "PK-mark timer expired (ClearPlayerkillingMarks stub — full PK subsystem deferred)"
            );
        }

        // C++ `ProcessCreatures` death safety (`crmain.cc:1113–1117`).
        // `apply_creature_death` is idempotent (returns early if creature gone).
        for cid in std::mem::take(&mut self.scratch_dead) {
            if self
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().health <= 0)
            {
                self.apply_creature_death(cid);
            }
        }
    }
}

#[cfg(test)]
#[path = "creature_think_tests.rs"]
mod tests;
