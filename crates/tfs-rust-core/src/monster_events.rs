//! Monster creature appear/move reactions and viewport fan-out.
//!
//! - `Monster::onCreatureMove` — `monster.cpp` (~212).
//! - `Monster::onCreatureAppear` — `monster.cpp` (~159–166).
//! - `Map::getSpectators` move fan-out — `map.cpp` (~264–323, ~386–474).

use tfs_rust_common::Position;
use slotmap::Key;

use crate::creature::CreatureKind;
use crate::game_world::{creature_can_see, GameWorld};
use crate::ids::CreatureId;
use crate::monster_ai::MAP_MAX_VIEWPORT;

impl GameWorld {
    pub fn monster_on_creature_appear_self(&mut self, cid: CreatureId) {
        self.monster_update_target_list(cid);
        self.monster_update_idle_status(cid);
        self.monster_try_acquire_chase_target(cid, None);
    }
    /// TFS `Map::getSpectators` multifloor Z span — `map.cpp` ~444–462.
    fn spectator_z_range(center_z: u8, multifloor: bool) -> std::ops::RangeInclusive<u8> {
        if !multifloor {
            return center_z..=center_z;
        }
        if center_z > 7 {
            let min_z = center_z.saturating_sub(2);
            let max_z = (center_z + 2).min(15);
            return min_z..=max_z;
        }
        if center_z == 6 {
            return 0..=8;
        }
        if center_z == 7 {
            return 0..=9;
        }
        0..=7
    }

    /// C++ `Map::getSpectators` — spatial viewport box only (`map.cpp` ~386–474).
    /// Used for move/appear fan-out; per-creature `canSee` is checked in `Monster::onCreatureMove`.
    fn collect_spatial_spectators(&self, center: Position, multifloor: bool) -> Vec<CreatureId> {
        let mut out = Vec::new();
        for z in Self::spectator_z_range(center.z, multifloor) {
            self.map.grid.collect_spectators(
                center.x,
                center.y,
                z,
                MAP_MAX_VIEWPORT,
                MAP_MAX_VIEWPORT,
                &mut out,
            );
        }
        out.sort_by_key(|id| id.data().as_ffi());
        out.dedup();
        out
    }

    /// Creatures within `Creature::canSee` of `center` (monster `updateTargetList` / spawn scan).
    pub(crate) fn collect_creature_spectators(&mut self, center: Position, multifloor: bool) -> Vec<CreatureId> {
        let range = i32::from(MAP_MAX_VIEWPORT);
        self.collect_spatial_spectators(center, multifloor)
            .into_iter()
            .filter(|&other| {
                let Some(other_pos) = self.creatures.get(other).map(|k| k.position()) else {
                    return false;
                };
                creature_can_see(center, other_pos, range, range)
            })
            .collect()
    }
    /// Monsters that should receive `Monster::onCreatureMove` for a move (`map.cpp` ~264–323).
    fn monsters_witnessing_move(&mut self, old_pos: Position, new_pos: Position) -> Vec<CreatureId> {
        let mut ids: Vec<CreatureId> = self
            .collect_spatial_spectators(old_pos, true)
            .into_iter()
            .chain(self.collect_spatial_spectators(new_pos, true))
            .filter(|&id| {
                self.creatures
                    .get(id)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            })
            .collect();
        ids.sort_by_key(|id| id.data().as_ffi());
        ids.dedup();
        ids
    }

    /// TFS `Monster::onCreatureMove` — `monster.cpp` ~212.
    pub fn monster_on_creature_move(
        &mut self,
        monster_id: CreatureId,
        creature_id: CreatureId,
        old_pos: Position,
        new_pos: Position,
    ) {
        if !self.creatures.contains_key(monster_id) {
            return;
        }

        if creature_id == monster_id {
            self.monster_update_target_list(monster_id);
            self.monster_update_idle_status(monster_id);
            return;
        }

        let monster_pos = match self.creatures.get(monster_id) {
            Some(k) => k.position(),
            None => return,
        };
        let range = i32::from(MAP_MAX_VIEWPORT);
        let can_see_new = creature_can_see(monster_pos, new_pos, range, range);
        let can_see_old = creature_can_see(monster_pos, old_pos, range, range);

        if can_see_new && !can_see_old {
            self.monster_on_creature_found(monster_id, creature_id, true);
        } else if !can_see_new && can_see_old {
            self.monster_remove_creature_from_lists(monster_id, creature_id);
        }

        self.monster_update_idle_status(monster_id);

        let (is_summon, follow, has_path) = match self.creatures.get(monster_id) {
            Some(CreatureKind::Monster(m)) => (
                m.base.is_summon(),
                m.base.follow_target,
                m.base.has_follow_path,
            ),
            _ => return,
        };

        if follow == Some(creature_id) {
            self.monster_on_follow_creature_moved(monster_id, creature_id, new_pos, has_path);
            let target_visible = self
                .creatures
                .get(creature_id)
                .map(|k| creature_can_see(monster_pos, k.position(), range, range))
                .unwrap_or(false);
            if new_pos.z != old_pos.z || !target_visible {
                if let Some(k) = self.creatures.get_mut(monster_id) {
                    if k.base().follow_target == Some(creature_id) {
                        k.base_mut().clear_follow_for_target(creature_id);
                    }
                    if k.base().attack_target == Some(creature_id) {
                        k.base_mut().clear_attack_for_target(creature_id);
                    }
                }
            }
            return;
        }

        // TFS `Monster::onCreatureMove` — `monster.cpp` ~287–289: `selectTarget(creature)` only
        // when we have no follow; no `searchTarget` on every move. Requires line-of-sight to the
        // mover's new tile (spatial spectators alone can include off-screen monsters).
        if !is_summon
            && can_see_new
            && self.monster_is_opponent(monster_id, creature_id)
            && follow.is_none()
        {
            self.monster_ensure_opponent_listed(monster_id, creature_id);
            let selected = self.monster_select_target(monster_id, creature_id);
        }
    }

    /// TFS `Creature::onCreatureMove` follow-target branch — `creature.cpp` ~619–637.
    ///
    /// 772 does not gate this on a path flag: `IdleStimulus` enqueues fresh `ToDoGo` when the
    /// target moves (`crnonpl.cc` via `SearchFlightField`). TFS uses `hasFollowPath`; repath when
    /// still chasing (see `PROTOCOL_VERSIONING.md` §12.1). Era split via
    /// [`MechanicsProfile::follow_repath_without_path`].
    fn monster_on_follow_creature_moved(
        &mut self,
        monster_id: CreatureId,
        creature_id: CreatureId,
        new_pos: Position,
        has_path: bool,
    ) {
        if !self.creatures.get(monster_id).is_some_and(|k| k.base().follow_target.is_some()) {
            return;
        }
        if !has_path && !self.mechanics.profile.follow_repath_without_path {
            return;
        }
        if self
            .creatures
            .get(monster_id)
            .is_some_and(|k| k.base().is_updating_path)
        {
            return;
        }

        // Hysteresis check: Only repath if target is no longer a valid goal from
        // the end of our current walk queue, or if sight is blocked.
        let should_repath = if self.beat_driven_loop {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
                return;
            };
            if m.base.walk_queue.is_empty() {
                true
            } else {
                let mut expected_pos = m.base.position;
                for &dir in m.base.walk_queue.iter().rev() {
                    expected_pos = expected_pos.offset(dir);
                }
                let target_distance = self.monster_effective_target_distance(m.target_distance);
                let expected_dist = crate::monster_ai::chebyshev(expected_pos, new_pos);
                let wrong_distance = if target_distance <= 1 {
                    expected_dist > 1
                } else {
                    expected_dist != target_distance
                };
                wrong_distance || !self.map.is_sight_clear(expected_pos, new_pos)
            }
        } else {
            true
        };

        if !should_repath {
            return;
        }

        // Do not skip repath when the follow *target moved*: `getPathTo` can return an empty
        // list while still inside the monster's keep-distance band, which left monsters frozen
        // after the player kited away from melee.
        if let Some(k) = self.creatures.get_mut(monster_id) {
            let base = k.base_mut();
            base.walk_queue.clear();
            base.walk_update_ticks = 0;
            base.is_updating_path = false;
            base.force_update_follow_path = true;
        }
        if self.beat_driven_loop {
            self.request_idle_stimulus(monster_id);
        } else {
            self.monster_follow_repath_now(monster_id);
        }
    }
    /// TFS `Monster::onFollowCreatureComplete` — `monster.cpp` ~599.
    pub(crate) fn monster_on_follow_creature_complete(&mut self, cid: CreatureId, target_id: CreatureId) {
        let (has_path, is_summon) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.base.has_follow_path, m.base.is_summon()),
            _ => return,
        };
        let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) else {
            return;
        };
        let idx = m.opponent_ids.iter().position(|&id| id == target_id);
        let Some(idx) = idx else {
            return;
        };
        m.opponent_ids.remove(idx);
        if has_path {
            m.opponent_ids.insert(0, target_id);
        } else if !is_summon {
            m.opponent_ids.push(target_id);
        }
    }
    /// TFS `Monster::onCreatureEnter` via `onCreatureAppear` spectator fan-out — `monster.cpp` ~435.
    pub fn monster_notify_creature_enter_viewport(&mut self, creature_id: CreatureId, pos: Position) {
        let monsters: Vec<CreatureId> = self
            .collect_spatial_spectators(pos, true)
            .into_iter()
            .filter(|&id| {
                id != creature_id
                    && self
                        .creatures
                        .get(id)
                        .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
            })
            .collect();
        self.monster_viewport_notify_depth += 1;
        for monster_id in monsters {
            // C++ `Monster::onCreatureAppear` → `onCreatureEnter` for each spatial spectator (`monster.cpp` ~167).
            self.monster_on_creature_found(monster_id, creature_id, true);
        }
        self.monster_viewport_notify_depth =
            self.monster_viewport_notify_depth.saturating_sub(1);
    }

    /// Notify monsters near a creature move (`Map::moveCreature` spectator fan-out).
    pub fn monster_dispatch_creature_move(
        &mut self,
        moved: CreatureId,
        old_pos: Position,
        new_pos: Position,
    ) {
        let monsters = self.monsters_witnessing_move(old_pos, new_pos);
        let witness_count = monsters.len();
        let moved_is_player = self
            .creatures
            .get(moved)
            .is_some_and(|k| matches!(k, CreatureKind::Player(_)));
        if moved_is_player {
        }
        for monster_id in monsters {
            self.monster_on_creature_move(monster_id, moved, old_pos, new_pos);
        }
    }
}
