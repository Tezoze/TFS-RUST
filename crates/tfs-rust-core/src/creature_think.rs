//! Creature think cadence — TFS `Game::checkCreatures` → `Creature::onThink` dispatch.
//!
//! - `Game::checkCreatures` — `game.cpp` (~3819).
//! - `Creature::onThink` — `creature.cpp` (~123).
//! - `Creature::onAttacking` / `Monster::doAttacking` — `creature.cpp` (~172), `monster.cpp` (~806).
//! - `Monster::onThink` / `Npc::onThink` — `monster.cpp` (~732), `npc.cpp` (~606).

use std::time::{Duration, Instant};

use rand::Rng;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// TFS `creature.h` `EVENT_CREATURE_THINK_INTERVAL`.
pub const EVENT_CREATURE_THINK_INTERVAL_MS: u32 = 1000;

/// TFS `creature.h` `EVENT_CREATURECOUNT` — bucket count for staggered checks.
pub const EVENT_CREATURECOUNT: u32 = 10;

/// TFS `creature.h` `EVENT_CHECK_CREATURE_INTERVAL` = think interval / bucket count.
pub const EVENT_CHECK_CREATURE_INTERVAL_MS: u32 =
    EVENT_CREATURE_THINK_INTERVAL_MS / EVENT_CREATURECOUNT;

/// Ms between follow path recomputes when following (`creature.cpp` ~153).
const FOLLOW_PATH_UPDATE_INTERVAL_MS: u32 = 200;

impl GameWorld {
    /// TFS `Game::addCreatureCheck` — random bucket assignment (`game.cpp` ~3798).
    pub(crate) fn add_creature_think_check(&mut self, cid: CreatureId) {
        let Some(k) = self.creatures.get_mut(cid) else {
            return;
        };
        if k.base().health <= 0 {
            return;
        }
        let bucket = rand::thread_rng().gen_range(0..EVENT_CREATURECOUNT) as u8;
        k.base_mut().think_check_bucket = Some(bucket);
    }

    /// TFS `Game::removeCreatureCheck` — idle / removed creatures skip think sweeps.
    pub(crate) fn remove_creature_think_check(&mut self, cid: CreatureId) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().think_check_bucket = None;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_creature_think_check_bucket(&mut self, cid: CreatureId, bucket: u8) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().think_check_bucket = Some(bucket % EVENT_CREATURECOUNT as u8);
        }
    }

    /// TFS `Game::checkCreatures` — one bucket every 100 ms, full cycle 1 s (`game.cpp` ~3819).
    pub fn check_creatures(&mut self, now: Instant) {
        let Some(last) = self.last_creature_bucket_tick else {
            self.last_creature_bucket_tick = Some(now);
            return;
        };

        if now.duration_since(last)
            < Duration::from_millis(u64::from(EVENT_CHECK_CREATURE_INTERVAL_MS))
        {
            return;
        }

        self.last_creature_bucket_tick = Some(now);

        let bucket = self.check_creature_bucket_index;
        self.check_creature_bucket_index =
            (self.check_creature_bucket_index + 1) % EVENT_CREATURECOUNT;

        let interval_ms = EVENT_CREATURE_THINK_INTERVAL_MS;
        let bucket_u8 = bucket as u8;

        let ids: Vec<CreatureId> = self
            .creatures
            .iter()
            .filter(|(_, k)| {
                matches!(k, CreatureKind::Monster(_) | CreatureKind::Npc(_))
                    && k.base().think_check_bucket == Some(bucket_u8)
            })
            .map(|(id, _)| id)
            .collect();

        for cid in ids {
            if !self.creature_alive_for_think(cid) {
                continue;
            }

            match self.creatures.get(cid) {
                Some(CreatureKind::Monster(_)) => {
                    self.monster_on_think(cid, interval_ms);
                    // C++ `checkCreatures`: `onAttacking` after `onThink` (`game.cpp` ~3837–3840).
                    if self.creature_alive_for_think(cid) {
                        self.creature_on_attacking(cid, interval_ms);
                    }
                }
                Some(CreatureKind::Npc(_)) => self.npc_on_think(cid, interval_ms),
                _ => continue,
            }
        }
    }

    /// 772 `ProcessCreatures` — regen + death safety only (`crmain.cc:1075–1138`).
    ///
    /// **Not** an AI think sweep. C++ `ProcessCreatures` does HP/mana regen (via `SKILL_FED`),
    /// player `CheckState` / logout marks, and a death safety net (`HP <= 0 && !IsDead → Death()`).
    /// It does **not** call `onThink`, `IdleStimulus`, target validation, or idle status updates —
    /// monster AI is driven entirely by the ToDoQueue / `IdleStimulus` / `CreatureMoveStimulus` /
    /// `DamageStimulus`. Calling `monster_on_think` here caused premature target loss and sleep
    /// transitions (audit RC1 — `docs/TFS-RUST_772_Monster_AI_Transition_Audit.md`).
    ///
    /// Regen is already handled by `process_skills_772` → `process_player_fed_regen_772`
    /// (`process_skills.rs:29`). Logout is handled by `process_connections_772` /
    /// `pending_idle_kick_772`. This function only retains the death safety net.
    pub fn process_creatures_772(&mut self) {
        // C++ iterates all creatures (`FirstFreeCreature`), not just think-bucketed ones.
        let ids: Vec<CreatureId> = self.creatures.iter().map(|(id, _)| id).collect();

        for cid in ids {
            // C++ `ProcessCreatures` death safety (`crmain.cc:1113–1117`):
            //   if(!Creature->IsDead && Creature->Skills[SKILL_HITPOINTS]->Get() <= 0){
            //       error(...); Creature->Death();
            //   }
            // `apply_creature_death` is idempotent (returns early if creature gone).
            let hp = self.creatures.get(cid).map(|k| k.base().health).unwrap_or(0);
            if hp <= 0 && self.creatures.contains_key(cid) {
                self.apply_creature_death(cid);
            }
        }
    }

    /// Whether `cid` should receive `onThink` this sweep (C++ `getHealth() > 0` gate).
    fn creature_alive_for_think(&self, cid: CreatureId) -> bool {
        self.creatures.get(cid).is_some_and(|k| k.base().health > 0)
    }

    /// TFS `Creature::onThink` — shared base logic for all creature kinds (D.2 subset).
    pub fn creature_on_think(&mut self, cid: CreatureId, interval_ms: u32) {
        let (follow, attack, master) = match self.creatures.get(cid) {
            Some(k) => (
                k.base().follow_target,
                k.base().attack_target,
                k.base().master,
            ),
            None => return,
        };

        if let Some(follow_id) = follow {
            if master != Some(follow_id) && !self.can_see_creature(cid, follow_id) {
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().clear_follow_for_target(follow_id);
                }
            }
        }

        if let Some(attack_id) = attack {
            if master != Some(attack_id) && !self.can_see_creature(cid, attack_id) {
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().clear_attack_for_target(attack_id);
                }
            }
        }

        let follow_id = self.creatures.get(cid).and_then(|k| k.base().follow_target);
        let skip_repath_at_goal =
            follow_id.is_some_and(|fid| self.monster_should_skip_follow_repath(cid, fid));

        if !self.beat_driven_loop {
            let mut run_follow_repath = false;
            if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                if let Some(_follow_id) = base.follow_target {
                    base.walk_update_ticks = base.walk_update_ticks.saturating_add(interval_ms);
                    let wants_repath = base.force_update_follow_path
                        || base.walk_update_ticks >= FOLLOW_PATH_UPDATE_INTERVAL_MS;
                    if wants_repath {
                        base.walk_update_ticks = 0;
                        base.force_update_follow_path = false;
                        if skip_repath_at_goal {
                            base.has_follow_path = true;
                        } else {
                            base.is_updating_path = true;
                        }
                    }
                }
                run_follow_repath = base.is_updating_path;
                if run_follow_repath {
                    base.is_updating_path = false;
                }
            }
            if run_follow_repath {
                self.go_to_follow_creature(cid, Some("think_repath"));
            }
        }

        self.events.on_think(cid, interval_ms);
    }

    /// TFS `Creature::onAttacking` — `creature.cpp` (~172–189).
    pub fn creature_on_attacking(&mut self, cid: CreatureId, interval_ms: u32) {
        let (attack_id, is_summon) = match self.creatures.get(cid) {
            Some(k) => (k.base().attack_target, k.base().is_summon()),
            None => return,
        };
        let Some(attack_id) = attack_id else {
            return;
        };
        if is_summon && attack_id == cid {
            return;
        }
        if !self.creatures.contains_key(attack_id) {
            return;
        }

        // TODO: `onAttacked` / target `onAttacked` callbacks (`creature.cpp` ~178–179).

        let (my_pos, target_pos) = match self.creatures.get(cid).zip(self.creatures.get(attack_id))
        {
            Some((attacker, target)) => (attacker.position(), target.position()),
            None => return,
        };
        if !self.monster_sight_clear(my_pos, target_pos) {
            return;
        }

        if self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
        {
            self.monster_do_attacking(cid, interval_ms);
        }
    }

    /// TFS `Monster::onThink` — base think + native AI (D.4).
    pub fn monster_on_think(&mut self, cid: CreatureId, interval_ms: u32) {
        self.creature_on_think(cid, interval_ms);
        self.monster_native_on_think(cid, interval_ms);
    }

    /// TFS `Npc::onThink` — base think + stub for idle walk / focus (D.6).
    pub fn npc_on_think(&mut self, cid: CreatureId, interval_ms: u32) {
        self.creature_on_think(cid, interval_ms);
        // D.6: random step within master_radius, focus / turn-to-speaker.
        let _ = interval_ms;
        let _ = cid;
    }
}

#[cfg(test)]
#[path = "creature_think_tests.rs"]
mod tests;
