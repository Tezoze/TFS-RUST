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
        // C++ iterates all creatures (`FirstFreeCreature`), not just think-bucketed ones.
        let ids: Vec<CreatureId> = self.creatures.iter().map(|(id, _)| id).collect();
        let round_nr = self.round_nr;

        // Collect players that need a stats packet after regen (deferred to avoid
        // borrowing `self` during iteration).
        let mut stats_dirty: Vec<CreatureId> = Vec::new();
        // Collect players whose PK-mark timer expired (stub — logged only).
        let mut pk_marks_expired: Vec<CreatureId> = Vec::new();

        for cid in &ids {
            // C++ `ProcessCreatures` item regen (`crmain.cc:1087-1095`):
            //   RegenInterval = Skills[SKILL_FED]->Get();  // food_level (Act)
            //   if(RegenInterval > 0 && (RoundNr % RegenInterval) == 0
            //      && !IsDead && !IsProtectionZone(pos))
            //       HP += 1; Mana += 4; SendPlayerData();
            let regen_info: Option<(i32, i32, bool, tfs_rust_common::Position)> =
                self.creatures.get(*cid).and_then(|k| match k {
                    CreatureKind::Player(p) => Some((
                        p.food_level,
                        p.base.health,
                        p.base.health > 0,
                        p.base.position,
                    )),
                    CreatureKind::Monster(m) => Some((0, m.base.health, m.base.health > 0, m.base.position)),
                    CreatureKind::Npc(_) => None,
                });

            if let Some((food_level, _hp, alive, pos)) = regen_info {
                if food_level > 0
                    && alive
                    && round_nr.is_multiple_of(food_level as u32)
                    && !self.tile_in_protection_zone(pos)
                {
                    if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(*cid) {
                        p.base.health = (p.base.health + 1).min(p.base.max_health);
                        p.mana = (p.mana + 4).min(p.max_mana);
                        stats_dirty.push(*cid);
                    }
                }
            }

            // C++ PK-mark clearing (`crmain.cc:1102-1105`):
            //   if(EarliestLogoutRound != 0 && EarliestLogoutRound <= RoundNr)
            //       ClearPlayerkillingMarks(); EarliestLogoutRound = 0;
            if let Some(CreatureKind::Player(p)) = self.creatures.get(*cid) {
                if p.earliest_logout_round != 0 && p.earliest_logout_round <= round_nr {
                    pk_marks_expired.push(*cid);
                }
            }
        }

        // Apply PK-mark clearing (stub — full PK subsystem deferred).
        for cid in pk_marks_expired {
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.earliest_logout_round = 0;
            }
            tracing::debug!(
                ?cid,
                round_nr,
                "PK-mark timer expired (ClearPlayerkillingMarks stub — full PK subsystem deferred)"
            );
        }

        // Send stats updates for players that gained HP/mana from item regen.
        for cid in stats_dirty {
            self.send_player_stats(cid);
        }

        // C++ `ProcessCreatures` death safety (`crmain.cc:1113–1117`):
        //   if(!Creature->IsDead && Creature->Skills[SKILL_HITPOINTS]->Get() <= 0){
        //       error(...); Creature->Death();
        //   }
        // `apply_creature_death` is idempotent (returns early if creature gone).
        for cid in ids {
            let hp = self.creatures.get(cid).map(|k| k.base().health).unwrap_or(0);
            if hp <= 0 && self.creatures.contains_key(cid) {
                self.apply_creature_death(cid);
            }
        }
    }
}

#[cfg(test)]
#[path = "creature_think_tests.rs"]
mod tests;
