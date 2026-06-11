//! Monster friend/opponent lists, target search/select, idle status.
//!
//! - `Monster::updateTargetList` — `monster.cpp` (~366).
//! - `Monster::searchTarget` / `selectTarget` — `monster.cpp` (~517, ~662).
//! - `Monster::updateIdleStatus` / `setIdle` — `monster.cpp` (~700–711).
//! - `Monster::canUseAttack` / `isTarget` — `monster.cpp` (~649, ~876).

use tfs_rust_common::enums::ZoneType;
use tfs_rust_common::Position;
use tfs_rust_content::monsters::MonsterSpellNode;

use crate::creature::CreatureKind;
use crate::game_world::{creature_can_see, GameWorld};
use crate::ids::CreatureId;
use crate::monster_ai::{chebyshev, manhattan, MAP_MAX_VIEWPORT};
use crate::player_flags::{flags_for_group, has_player_flag, PLAYER_FLAG_IGNORED_BY_MONSTERS};

/// TFS `TargetSearchType_t` (`monster.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSearchType {
    Default,
    Nearest,
    AttackRange,
    Random,
    /// Lowest-health opponent (772 `Strategy` weakest bucket / TFS `<targetstrategy>` health).
    /// The HP metric (current vs max) is profile-driven (B3.1, `WeakestTargetMetric`).
    HealthLow,
}

fn spell_in_attack_range(spell: &MonsterSpellNode, distance: u32) -> bool {
    let range = spell
        .attributes
        .get("range")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    if range == 0 {
        spell.element.eq_ignore_ascii_case("melee") && distance <= 1
    } else {
        distance <= range
    }
}

impl GameWorld {
    pub(crate) fn monster_weakest_opponent(&self, candidates: &[CreatureId]) -> Option<CreatureId> {
        let metric = self.mechanics.profile.weakest_target_metric;
        let mut best: Option<(CreatureId, i32)> = None;
        for &oid in candidates {
            let Some(k) = self.creatures.get(oid) else {
                continue;
            };
            let base = k.base();
            let hp = match metric {
                crate::formulas::WeakestTargetMetric::CurrentHp => base.health,
                crate::formulas::WeakestTargetMetric::MaxHp => base.max_health,
            };
            if best.map(|(_, b)| hp < b).unwrap_or(true) {
                best = Some((oid, hp));
            }
        }
        best.map(|(id, _)| id)
    }
    /// TFS `Monster::updateTargetList` — `monster.cpp` ~366.
    pub fn monster_update_target_list(&mut self, cid: CreatureId) {
        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return,
        };

        self.monster_prune_creature_lists(cid);

        let spectators = self.collect_creature_spectators(pos, true);
        for other in spectators {
            if other == cid {
                continue;
            }
            self.monster_on_creature_found(cid, other, false);
        }
    }
    pub(crate) fn monster_remove_creature_from_lists(&mut self, monster_id: CreatureId, creature_id: CreatureId) {
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
            m.opponent_ids.retain(|&id| id != creature_id);
            m.friend_ids.retain(|&id| id != creature_id);
        }
        self.monster_update_idle_status(monster_id);
        self.monster_maybe_walk_to_spawn(monster_id);
    }

    pub(crate) fn monster_prune_creature_lists(&mut self, cid: CreatureId) {
        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return,
        };
        let (mut opponents, mut friends) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.opponent_ids.clone(), m.friend_ids.clone()),
            _ => return,
        };

        opponents.retain(|&oid| self.monster_creature_visible_to(cid, pos, oid));
        friends.retain(|&fid| self.monster_creature_visible_to(cid, pos, fid));

        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.opponent_ids = opponents;
            m.friend_ids = friends;
        }
    }

    pub(crate) fn monster_creature_visible_to(&self, viewer: CreatureId, viewer_pos: Position, other: CreatureId) -> bool {
        let Some(other_kind) = self.creatures.get(other) else {
            return false;
        };
        if other_kind.base().health <= 0 {
            return false;
        }
        if !self.can_see_creature(viewer, other) {
            return false;
        }
        let op = other_kind.position();
        creature_can_see(
            viewer_pos,
            op,
            i32::from(MAP_MAX_VIEWPORT),
            i32::from(MAP_MAX_VIEWPORT),
        )
    }

    /// TFS `Monster::onCreatureFound` — `monster.cpp` ~414.
    pub(crate) fn monster_on_creature_found(&mut self, monster_id: CreatureId, creature_id: CreatureId, push_front: bool) {
        if creature_id == monster_id {
            return;
        }
        let pos = match self.creatures.get(monster_id) {
            Some(k) => k.position(),
            None => return,
        };
        let creature_pos = match self.creatures.get(creature_id) {
            Some(k) => k.position(),
            None => return,
        };
        if !self.can_see_creature(monster_id, creature_id) {
            return;
        }
        if !creature_can_see(
            pos,
            creature_pos,
            i32::from(MAP_MAX_VIEWPORT),
            i32::from(MAP_MAX_VIEWPORT),
        ) {
            return;
        }

        if self.monster_is_friend(monster_id, creature_id) {
            self.monster_add_friend(monster_id, creature_id);
        }
        if self.monster_is_opponent(monster_id, creature_id) {
            self.monster_add_opponent(monster_id, creature_id, push_front);
        }
        self.monster_update_idle_status(monster_id);
        // Already-active monsters (not via `set_idle` wake) still need chase scheduling.
        if push_front {
            self.monster_schedule_chase_after_opponent_add(monster_id, Some(creature_id));
        }
        // C++ `Monster::onCreatureFound` stops here (`monster.cpp` ~414) — no `searchTarget` /
        // `setFollowCreature` on enter. Chase is acquired from `onThink` / move handlers only;
        // synchronous acquire on login fan-out ran A* for every viewport monster (~4s Forgotten).
    }

    /// Chase scheduling after a new opponent enters the list (viewport / move-enter).
    pub(crate) fn monster_schedule_chase_after_opponent_add(
        &mut self,
        monster_id: CreatureId,
        preferred: Option<CreatureId>,
    ) {
        let should = self.creatures.get(monster_id).is_some_and(|k| {
            matches!(k, CreatureKind::Monster(m) if {
                !m.base.is_summon()
                    && m.base.follow_target.is_none()
                    && !m.opponent_ids.is_empty()
            })
        });
        if !should {
            return;
        }
        if self.monster_viewport_notify_depth > 0 {
            let bucket = self.check_creature_bucket_index as u8;
            if self
                .creatures
                .get(monster_id)
                .is_some_and(|k| k.base().think_check_bucket.is_none())
            {
                self.add_creature_think_check(monster_id);
            }
            if let Some(k) = self.creatures.get_mut(monster_id) {
                k.base_mut().think_check_bucket = Some(bucket);
            }
        } else {
            self.monster_try_acquire_chase_target(monster_id, preferred);
        }
    }

    /// Acquire `follow_target` immediately when a player/opponent enters range — do not wait for `onThink`.
    pub(crate) fn monster_try_acquire_chase_target(
        &mut self,
        monster_id: CreatureId,
        preferred: Option<CreatureId>,
    ) {
        if self.creatures.get(monster_id).is_some_and(|k| {
            matches!(k, CreatureKind::Monster(m) if m.base.is_summon()) || k.base().follow_target.is_some()
        }) {
            return;
        }
        if let Some(pid) = preferred {
            if self.monster_is_opponent(monster_id, pid) {
                self.monster_ensure_opponent_listed(monster_id, pid);
            }
            if self.monster_select_target(monster_id, pid) {
                return;
            }
        }
        self.monster_search_target(monster_id, TargetSearchType::Default);
    }

    pub(crate) fn monster_is_friend(&self, monster_id: CreatureId, creature_id: CreatureId) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
            return false;
        };
        if m.base.is_summon() {
            return false;
        }
        matches!(self.creatures.get(creature_id), Some(CreatureKind::Monster(other)) if !other.base.is_summon())
    }

    pub(crate) fn monster_is_opponent(&self, monster_id: CreatureId, creature_id: CreatureId) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
            return false;
        };
        if m.base.is_summon() {
            let master = m.base.master;
            return master != Some(creature_id);
        }
        match self.creatures.get(creature_id) {
            Some(CreatureKind::Player(p)) => {
                if p.ghost_mode {
                    return false;
                }
                let flags = flags_for_group(&self.groups, p.group_id);
                !has_player_flag(flags, PLAYER_FLAG_IGNORED_BY_MONSTERS)
            }
            Some(other) if other.base().is_summon() => other
                .base()
                .master
                .and_then(|mid| self.creatures.get(mid))
                .is_some_and(|master| matches!(master, CreatureKind::Player(_))),
            _ => false,
        }
    }

    pub(crate) fn monster_add_friend(&mut self, monster_id: CreatureId, friend_id: CreatureId) {
        let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) else {
            return;
        };
        if !m.friend_ids.contains(&friend_id) {
            m.friend_ids.push(friend_id);
        }
    }

    /// Ensure `opponent_id` is in the monster target list before `selectTarget` / move-acquire paths.
    pub(crate) fn monster_ensure_opponent_listed(&mut self, monster_id: CreatureId, opponent_id: CreatureId) {
        let already = self.creatures.get(monster_id).is_some_and(|k| {
            matches!(k, CreatureKind::Monster(m) if m.opponent_ids.contains(&opponent_id))
        });
        if !already {
            self.monster_add_opponent(monster_id, opponent_id, true);
            self.monster_update_idle_status(monster_id);
        }
    }

    pub(crate) fn monster_add_opponent(&mut self, monster_id: CreatureId, opponent_id: CreatureId, push_front: bool) {
        let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) else {
            return;
        };
        if m.opponent_ids.contains(&opponent_id) {
            return;
        }
        if push_front {
            m.opponent_ids.insert(0, opponent_id);
        } else {
            m.opponent_ids.push(opponent_id);
        }
    }

    /// TFS `Monster::updateIdleStatus` / `setIdle` — `monster.cpp` ~700–711.
    pub fn monster_update_idle_status(&mut self, cid: CreatureId) {
        let idle = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                !m.base.is_summon() && m.opponent_ids.is_empty()
            }
            _ => return,
        };
        self.monster_set_idle(cid, idle);
    }

    pub(crate) fn monster_set_idle(&mut self, cid: CreatureId, idle: bool) {
        let became_idle = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) else {
                return;
            };
            if m.base.health <= 0 {
                return;
            }
            if m.is_idle == idle {
                return;
            }
            let was_idle = m.is_idle;
            m.is_idle = idle;
            if !idle && was_idle {
            }
            if idle {
                m.base.damage_map.clear();
                m.opponent_ids.clear();
                m.friend_ids.clear();
                m.base.clear_targets();
                m.base.has_follow_path = false;
                m.base.walk_queue.clear();
            }
            idle
        };
        if became_idle {
            self.remove_creature_think_check(cid);
        } else {
            self.add_creature_think_check(cid);
            if self.beat_driven_loop {
                self.request_idle_stimulus(cid);
            }
        }
    }

    /// TFS `Monster::isTarget` — `monster.cpp` ~649.
    pub(crate) fn monster_is_target(&self, monster_id: CreatureId, target_id: CreatureId) -> bool {
        let Some(monster_pos) = self.creatures.get(monster_id).map(|k| k.position()) else {
            return false;
        };
        let Some(target) = self.creatures.get(target_id) else {
            return false;
        };
        if target.base().health <= 0 {
            return false;
        }
        if !self.can_see_creature(monster_id, target_id) {
            return false;
        }
        let tp = target.position();
        if tp.z != monster_pos.z {
            return false;
        }
        if let Some(tile) = self.map.get_tile(tp) {
            if tile.body().zone == ZoneType::Protection {
                return false;
            }
        }
        true
    }

    /// TFS `Monster::canUseAttack` — `monster.cpp` ~876.
    pub fn monster_can_use_attack(&self, monster_id: CreatureId, pos: Position, target_id: CreatureId) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
            return false;
        };
        if !m.is_hostile {
            return true;
        }
        let target_pos = match self.creatures.get(target_id) {
            Some(k) => k.position(),
            None => return false,
        };
        let dist = chebyshev(pos, target_pos) as u32;
        let db_name = m.base.name.to_lowercase();
        let spells = self
            .monsters_db
            .monsters
            .get(&db_name)
            .map(|t| t.attack_spells.as_slice())
            .unwrap_or(&[]);
        for spell in spells {
            if spell_in_attack_range(spell, dist) && self.map.is_sight_clear(pos, target_pos) {
                return true;
            }
        }
        false
    }

    /// TFS `Monster::searchTarget` — `monster.cpp` ~517.
    pub fn monster_search_target(&mut self, monster_id: CreatureId, search_type: TargetSearchType) -> bool {
        let (pos, opponents, follow) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
                return false;
            };
            (m.base.position, m.opponent_ids.clone(), m.base.follow_target)
        };

        let mut result_list: Vec<CreatureId> = Vec::new();
        for &oid in &opponents {
            if follow == Some(oid) {
                continue;
            }
            if !self.monster_is_target(monster_id, oid) {
                continue;
            }
            if search_type == TargetSearchType::Random
                || self.monster_can_use_attack(monster_id, pos, oid)
            {
                result_list.push(oid);
            }
        }

        match search_type {
            TargetSearchType::HealthLow => {
                // B3.1 — pick the weakest reachable opponent. Metric (current vs max HP) is
                // profile-driven: 772 compares **current** HP (`crnonpl.cc` Strategy),
                // TFS compares **max** HP (`monsters.cpp` `<targetstrategy>`).
                if let Some(best) = self.monster_weakest_opponent(&result_list) {
                    return self.monster_select_target(monster_id, best);
                }
            }
            TargetSearchType::Nearest => {
                if !result_list.is_empty() {
                    let mut best = result_list[0];
                    let mut min_range = self
                        .creatures
                        .get(best)
                        .map(|k| manhattan(pos, k.position()))
                        .unwrap_or(i32::MAX);
                    for &oid in result_list.iter().skip(1) {
                        let Some(d) = self
                            .creatures
                            .get(oid)
                            .map(|k| manhattan(pos, k.position()))
                        else {
                            continue;
                        };
                        if d < min_range {
                            best = oid;
                            min_range = d;
                        }
                    }
                    return self.monster_select_target(monster_id, best);
                }
                let mut best: Option<(CreatureId, i32)> = None;
                for &oid in &opponents {
                    if !self.monster_is_target(monster_id, oid) {
                        continue;
                    }
                    let Some(d) = self
                        .creatures
                        .get(oid)
                        .map(|k| manhattan(pos, k.position()))
                    else {
                        continue;
                    };
                    if best.map(|(_, m)| d < m).unwrap_or(true) {
                        best = Some((oid, d));
                    }
                }
                if let Some((oid, _)) = best {
                    return self.monster_select_target(monster_id, oid);
                }
            }
            TargetSearchType::Default | TargetSearchType::Random | TargetSearchType::AttackRange => {
                if !result_list.is_empty() {
                    let idx = if result_list.len() == 1 {
                        0
                    } else {
                        rand::random::<usize>() % result_list.len()
                    };
                    return self.monster_select_target(monster_id, result_list[idx]);
                }
                if search_type == TargetSearchType::AttackRange {
                    return false;
                }
            }
        }

        for &oid in &opponents {
            if follow != Some(oid) && self.monster_select_target(monster_id, oid) {
                return true;
            }
        }
        false
    }

    /// TFS `Monster::selectTarget` — `monster.cpp` ~662.
    pub(crate) fn monster_select_target(&mut self, monster_id: CreatureId, target_id: CreatureId) -> bool {
        if !self.monster_is_target(monster_id, target_id) {
            return false;
        }
        let in_list = self
            .creatures
            .get(monster_id)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.opponent_ids.contains(&target_id)));
        if !in_list {
            return false;
        }

        if let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) {
            if m.is_hostile || m.base.is_summon() {
                if let Some(k) = self.creatures.get_mut(monster_id) {
                    k.base_mut().attack_target = Some(target_id);
                }
            }
        }

        let ret = self.monster_set_follow_creature(monster_id, Some(target_id));
        if ret {
            self.monster_update_look_direction(monster_id);
        }
        ret
    }
    /// TFS `Creature::setFollowCreature` — `creature.cpp` ~1058.
    pub(crate) fn monster_set_follow_creature(&mut self, monster_id: CreatureId, target: Option<CreatureId>) -> bool {
        let Some(target_id) = target else {
            if let Some(k) = self.creatures.get_mut(monster_id) {
                let base = k.base_mut();
                base.is_updating_path = false;
                base.follow_target = None;
                base.has_follow_path = false;
            }
            return true;
        };

        if self
            .creatures
            .get(monster_id)
            .and_then(|k| k.base().follow_target)
            == Some(target_id)
        {
            return true;
        }

        let (monster_pos, target_pos) = {
            let Some(mp) = self.creatures.get(monster_id).map(|k| k.position()) else {
                return false;
            };
            let Some(tp) = self.creatures.get(target_id).map(|k| k.position()) else {
                return false;
            };
            (mp, tp)
        };
        if !self.can_see_creature(monster_id, target_id)
            || !creature_can_see(
                monster_pos,
                target_pos,
                i32::from(MAP_MAX_VIEWPORT),
                i32::from(MAP_MAX_VIEWPORT),
            )
        {
            if let Some(k) = self.creatures.get_mut(monster_id) {
                k.base_mut().follow_target = None;
            }
            return false;
        }

        if let Some(k) = self.creatures.get_mut(monster_id) {
            let base = k.base_mut();
            if !base.walk_queue.is_empty() {
                base.walk_queue.clear();
            }
            base.has_follow_path = false;
            base.force_update_follow_path = false;
            base.follow_target = Some(target_id);
            base.is_updating_path = true;
        }
        if self.beat_driven_loop {
            self.request_idle_stimulus(monster_id);
        } else {
            self.monster_follow_repath_now(monster_id, Some("set_follow"));
        }
        true
    }
}
