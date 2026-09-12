//! Public monster appear-batch path — C++ `SpawnMonsterAppear` (`chase_kite_scenario.cc`).
//!
//! Sleep-until-damage lives on `GameWorld` (not `Monster`) so production spawn still wakes
//! via [`GameWorld::monster_on_creature_appear_self`].

use slotmap::Key;

use crate::creature::{CreatureKind, MonsterState};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::monster_ai::compute_look_toward_target;
use crate::walk::creature_turn_with_broadcast;

impl GameWorld {
    /// Keep `Sleeping` until first `DamageStimulus` — C++ scenario `monster_state sleeping`.
    /// Production spawn never calls this.
    pub fn preserve_monster_sleep_until_damage(&mut self, cid: CreatureId) {
        self.sleep_until_damage.insert(cid);
    }

    pub(crate) fn monster_sleeps_until_damage(&self, cid: CreatureId) -> bool {
        self.sleep_until_damage.contains(&cid)
            && self.creatures.get(cid).is_some_and(|k| {
                matches!(
                    k,
                    CreatureKind::Monster(m)
                        if m.state == MonsterState::Sleeping && m.is_idle
                )
            })
    }

    pub(crate) fn clear_sleep_until_damage(&mut self, cid: CreatureId) {
        self.sleep_until_damage.remove(&cid);
    }

    /// Appear without inline `IdleStimulus`, then batch `ToDoYield`.
    /// C++ `SpawnMonsterAppear` — `chase_kite_scenario.cc`.
    pub fn appear_monsters(&mut self, ids: &[CreatureId]) {
        for &cid in ids {
            self.appear_monster_without_idle(cid);
        }
        for &cid in ids {
            self.creature_todo_yield(cid);
        }
    }

    /// Wake monster and run appear/target acquisition — `monster_appear` scenario step.
    pub fn appear_monster(&mut self, cid: CreatureId) {
        if !self.monster_sleeps_until_damage(cid)
            && let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid)
        {
            m.is_idle = false;
        }
        self.monster_on_creature_appear_self(cid);
    }

    /// Appear step without inline `IdleStimulus` — C++ `SpawnMonsterAppear` defers yield to batch tail.
    pub(crate) fn appear_monster_without_idle(&mut self, monster_id: CreatureId) {
        let keep_sleeping = self.monster_sleeps_until_damage(monster_id);
        if !keep_sleeping && let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id)
        {
            m.is_idle = false;
            if m.state == MonsterState::Sleeping {
                m.state = MonsterState::Idle;
            }
        }
        self.monster_update_target_list(monster_id);
        if let Some(opponent) = self.creatures.get(monster_id).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            m.opponent_ids.first().copied()
        }) {
            self.acquire_chase_target_without_idle(monster_id, opponent);
        }
        self.appear_face_target_for_debug(monster_id);
    }

    /// Set follow/attack without `request_idle_stimulus` — appear-batch only.
    fn acquire_chase_target_without_idle(&mut self, monster_id: CreatureId, target_id: CreatureId) {
        if !self.monster_is_target(monster_id, target_id) {
            return;
        }
        let in_list = self.creatures.get(monster_id).is_some_and(
            |k| matches!(k, CreatureKind::Monster(m) if m.opponent_ids.contains(&target_id)),
        );
        if !in_list {
            return;
        }
        if !self.can_see_creature(monster_id, target_id) {
            return;
        }
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
            if m.is_hostile || m.base.is_summon() {
                m.base.attack_target = Some(target_id);
            }
            m.base.follow_target = Some(target_id);
            m.base.is_updating_path = true;
            m.base.has_follow_path = false;
            m.base.force_update_follow_path = false;
            if !m.base.walk_queue.is_empty() {
                m.base.walk_queue.clear();
                m.base.walk_destinations.clear();
            }
        }
    }

    /// Chase JSONL rotate @ tick 0 — bypasses `walk_timer_idle` gate on appear.
    fn appear_face_target_for_debug(&mut self, cid: CreatureId) {
        if !tracing::enabled!(target: "chase", tracing::Level::TRACE) {
            return;
        }
        let (pos, target_id, current) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                (m.base.position, m.base.attack_target, m.base.direction)
            }
            _ => return,
        };
        let Some(target_id) = target_id else {
            return;
        };
        let target_pos = match self.creatures.get(target_id) {
            Some(k) => k.position(),
            None => return,
        };
        let new_dir = compute_look_toward_target(pos, target_pos, current);
        if new_dir != current {
            creature_turn_with_broadcast(self, cid, new_dir);
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                tracing::trace!(
                    target: "chase",
                    event = "rotate",
                    tick = self.chase_trace_tick(),
                    id = cid.data().as_ffi(),
                    name = m.base.name.as_str(),
                    dir = new_dir as u8,
                    target_id = target_id.data().as_ffi(),
                );
            }
        }
    }
}
