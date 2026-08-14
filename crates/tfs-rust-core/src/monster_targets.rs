//! Monster friend/opponent lists, target search/select, idle status.
//!
//! - `Monster::updateTargetList` — `monster.cpp` (~366).
//! - `Monster::searchTarget` / `selectTarget` — `monster.cpp` (~517, ~662).
//! - `Monster::updateIdleStatus` / `setIdle` — `monster.cpp` (~700–711).
//! - `Monster::canUseAttack` / `isTarget` — `monster.cpp` (~649, ~876).

use rand::RngExt;
use tfs_rust_common::Position;
use tfs_rust_common::enums::ZoneType;
use tfs_rust_content::monsters::MonsterSpellNode;

use crate::creature::{monster_has_melee_strike, runtime_spell_in_attack_range};

use crate::creature::{CreatureKind, MonsterState};
use crate::game_world::{GameWorld, creature_can_see};
use crate::ids::CreatureId;
use crate::monster_ai::{MAP_MAX_VIEWPORT, chebyshev, manhattan};
use crate::player_flags::{PLAYER_FLAG_IGNORED_BY_MONSTERS, flags_for_group, has_player_flag};

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

pub(crate) fn monster_spell_is_melee(spell: &MonsterSpellNode) -> bool {
    spell.element.eq_ignore_ascii_case("melee")
        || spell
            .attributes
            .get("name")
            .is_some_and(|n| n.eq_ignore_ascii_case("melee"))
}

fn spell_in_attack_range(spell: &MonsterSpellNode, distance: u32) -> bool {
    let range = spell
        .attributes
        .get("range")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    if range == 0 {
        monster_spell_is_melee(spell) && distance <= 1
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
    pub(crate) fn monster_remove_creature_from_lists(
        &mut self,
        monster_id: CreatureId,
        creature_id: CreatureId,
    ) {
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
            m.opponent_ids.retain(|&id| id != creature_id);
            m.friend_ids.retain(|&id| id != creature_id);
        }
        self.monster_update_idle_status(monster_id);
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

    pub(crate) fn monster_creature_visible_to(
        &self,
        viewer: CreatureId,
        viewer_pos: Position,
        other: CreatureId,
    ) -> bool {
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
            self.mechanics.profile.underground_sees_surface,
        )
    }

    /// TFS `Monster::onCreatureFound` — `monster.cpp` ~414.
    pub(crate) fn monster_on_creature_found(
        &mut self,
        monster_id: CreatureId,
        creature_id: CreatureId,
        push_front: bool,
    ) {
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
            self.mechanics.profile.underground_sees_surface,
        ) {
            return;
        }

        if self.monster_is_friend(monster_id, creature_id) {
            self.monster_add_friend(monster_id, creature_id);
        }
        if self.monster_is_opponent(monster_id, creature_id) {
            self.monster_add_opponent(monster_id, creature_id, push_front);
        }
        let preserve_sleep = self.creatures.get(monster_id).is_some_and(|k| {
            matches!(
                k,
                CreatureKind::Monster(m)
                    if m.harness_preserve_sleep
                        && m.state == MonsterState::Sleeping
                        && m.is_idle
            )
        });
        if !preserve_sleep {
            self.monster_update_idle_status(monster_id);
        }
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
        _preferred: Option<CreatureId>,
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
        // Suppress synchronous chase acquire during login fan-out: running A* for every
        // viewport monster took ~4s on Forgotten. The monster will be woken by a later
        // `IdleStimulus` once the fan-out completes (`monster_viewport_notify_depth == 0`).
        if self.monster_viewport_notify_depth == 0 {
            self.request_idle_stimulus(monster_id);
        }
    }

    pub(crate) fn monster_is_friend(
        &self,
        monster_id: CreatureId,
        creature_id: CreatureId,
    ) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
            return false;
        };
        if m.base.is_summon() {
            return false;
        }
        matches!(self.creatures.get(creature_id), Some(CreatureKind::Monster(other)) if !other.base.is_summon())
    }

    pub(crate) fn monster_is_opponent(
        &self,
        monster_id: CreatureId,
        creature_id: CreatureId,
    ) -> bool {
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
                // Monsters without SeeInvisible do not treat invisible players as opponents
                // (`Creature::canSeeCreature` + `crnonpl.cc:2514` acquire gate).
                if !m.see_invisible && p.base.is_invisible() {
                    return false;
                }
                let flags = flags_for_group(&self.groups, p.group_id);
                !has_player_flag(flags, PLAYER_FLAG_IGNORED_BY_MONSTERS)
            }
            Some(other) if other.base().is_summon() => {
                if !m.see_invisible && other.base().is_invisible() {
                    return false;
                }
                other
                    .base()
                    .master
                    .and_then(|mid| self.creatures.get(mid))
                    .is_some_and(|master| matches!(master, CreatureKind::Player(_)))
            }
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
    pub(crate) fn monster_ensure_opponent_listed(
        &mut self,
        monster_id: CreatureId,
        opponent_id: CreatureId,
    ) {
        let already = self.creatures.get(monster_id).is_some_and(
            |k| matches!(k, CreatureKind::Monster(m) if m.opponent_ids.contains(&opponent_id)),
        );
        if !already {
            self.monster_add_opponent(monster_id, opponent_id, true);
            self.monster_update_idle_status(monster_id);
        }
    }

    pub(crate) fn monster_add_opponent(
        &mut self,
        monster_id: CreatureId,
        opponent_id: CreatureId,
        push_front: bool,
    ) {
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
            Some(CreatureKind::Monster(m)) => !m.base.is_summon() && m.opponent_ids.is_empty(),
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
            if idle {
                m.state = MonsterState::Sleeping;
                m.is_idle = true;
            } else if was_idle || m.state == MonsterState::Sleeping {
                m.state = MonsterState::Idle;
                m.is_idle = false;
            }
            if idle {
                m.base.damage_map.clear();
                m.opponent_ids.clear();
                m.friend_ids.clear();
                m.base.clear_targets();
                m.base.has_follow_path = false;
                m.base.walk_queue.clear();
                m.base.walk_destinations.clear();
            }
            idle
        };
        if !became_idle {
            self.request_idle_stimulus(cid);
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

    /// Creature line-of-sight via `ThrowPossible` (major-axis interpolation +
    /// `UNTHROW`, `info.cc:1154`). All callers throw with `power = 0`
    /// (`crnonpl.cc:2798`).
    pub(crate) fn monster_sight_clear(&self, from: Position, to: Position) -> bool {
        self.map.throw_possible(from, to, 0)
    }

    /// TFS `Monster::canUseAttack` — `monster.cpp` ~876.
    pub fn monster_can_use_attack(
        &self,
        monster_id: CreatureId,
        pos: Position,
        target_id: CreatureId,
    ) -> bool {
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
        if !self.monster_sight_clear(pos, target_pos) {
            return false;
        }
        let dist = chebyshev(pos, target_pos) as u32;
        if monster_has_melee_strike(m.melee_skill, dist) {
            return true;
        }
        for spell in &m.spells {
            if runtime_spell_in_attack_range(spell, dist) {
                return true;
            }
        }
        // Stub spawns without combat config: fall back to content db for legacy tests.
        if m.melee_skill == 0 && m.spells.is_empty() {
            let db_name = m.base.name.to_lowercase();
            let spells = self
                .monsters_db
                .monsters
                .get(&db_name)
                .map(|t| t.attack_spells.as_slice())
                .unwrap_or(&[]);
            for spell in spells {
                if spell_in_attack_range(spell, dist) {
                    return true;
                }
            }
        }
        false
    }

    /// C++ `ThrowPossible` — ranged-only gate for 772 distance-fighting idle (`crnonpl.cc:2795-2797`).
    ///
    /// Melee at cheb=1 does **not** count; when this returns false the close branch runs
    /// (`melee_dance` at dist==1) even if [`Self::monster_can_use_attack`] is true.
    pub fn monster_throw_possible(
        &self,
        monster_id: CreatureId,
        pos: Position,
        target_id: CreatureId,
    ) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
            return false;
        };
        if !m.is_hostile {
            return false;
        }
        let target_pos = match self.creatures.get(target_id) {
            Some(k) => k.position(),
            None => return false,
        };
        if !self.monster_sight_clear(pos, target_pos) {
            return false;
        }
        let dist = chebyshev(pos, target_pos) as u32;
        for spell in &m.spells {
            if runtime_spell_in_attack_range(spell, dist) {
                return true;
            }
        }
        if m.spells.is_empty() {
            let db_name = m.base.name.to_lowercase();
            let spells = self
                .monsters_db
                .monsters
                .get(&db_name)
                .map(|t| t.attack_spells.as_slice())
                .unwrap_or(&[]);
            for spell in spells {
                if spell_in_attack_range(spell, dist) {
                    return true;
                }
            }
        }
        false
    }

    /// TFS `Monster::searchTarget` — `monster.cpp` ~517.
    pub fn monster_search_target(
        &mut self,
        monster_id: CreatureId,
        search_type: TargetSearchType,
    ) -> bool {
        let (pos, opponents, follow) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(monster_id) else {
                return false;
            };
            (
                m.base.position,
                m.opponent_ids.clone(),
                m.base.follow_target,
            )
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
            TargetSearchType::Default
            | TargetSearchType::Random
            | TargetSearchType::AttackRange => {
                if !result_list.is_empty() {
                    let idx = if result_list.len() == 1 {
                        0
                    } else {
                        rand::rng().random_range(0..result_list.len())
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
    pub(crate) fn monster_select_target(
        &mut self,
        monster_id: CreatureId,
        target_id: CreatureId,
    ) -> bool {
        if !self.monster_is_target(monster_id, target_id) {
            return false;
        }
        let in_list = self.creatures.get(monster_id).is_some_and(
            |k| matches!(k, CreatureKind::Monster(m) if m.opponent_ids.contains(&target_id)),
        );
        if !in_list {
            return false;
        }

        // 772: strategy / `selectTarget` only sets `Target` (= follow via `setFollowCreature`).
        // `Combat.AttackDest` + `AttackStimulus` are applied in idle walk `SetAttackDest`
        // (`crnonpl.cc:2540` vs `:2784`). Do not set `attack_target` here or that call
        // early-outs and skips the logout lock (`crcombat.cc:358-360`).
        self.monster_set_follow_creature(monster_id, Some(target_id))
    }

    /// TFS `Monster::challengeCreature` — `monster.cpp:2070`.
    /// PC-3a Phase 6: `doChallengeCreature` from `challenge.lua`.
    pub fn monster_challenge_creature(
        &mut self,
        monster_id: CreatureId,
        challenger_id: CreatureId,
    ) -> bool {
        let (is_summon, name) = match self.creatures.get(monster_id) {
            Some(CreatureKind::Monster(m)) => (m.base.is_summon(), m.base.name.clone()),
            _ => return false,
        };
        if is_summon {
            return false;
        }
        let challengeable = self
            .monsters_db
            .get_by_name(&name)
            .map(|t| t.flags.is_challengeable)
            .unwrap_or(true);
        if !challengeable {
            return false;
        }
        // Ensure challenger is on the opponent list so `selectTarget` can succeed.
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
            if !m.opponent_ids.contains(&challenger_id) {
                m.opponent_ids.push(challenger_id);
            }
        }
        if !self.monster_select_target(monster_id, challenger_id) {
            return false;
        }
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
            m.target_change_cooldown = 8000;
            m.challenge_focus_duration = 8000;
            m.target_change_ticks = 0;
        }
        true
    }

    /// Lua `doChallengeCreature(creature, target)` — challenger is the knight,
    /// target is the monster being challenged.
    pub fn lua_do_challenge_creature(&mut self, challenger_u64: u64, target_u64: u64) -> bool {
        let Some(challenger) = self.resolve_creature_u64(challenger_u64) else {
            return false;
        };
        let Some(target) = self.resolve_creature_u64(target_u64) else {
            return false;
        };
        self.monster_challenge_creature(target, challenger)
    }

    /// TFS `Creature::setFollowCreature` — `creature.cpp` ~1058.
    pub(crate) fn monster_set_follow_creature(
        &mut self,
        monster_id: CreatureId,
        target: Option<CreatureId>,
    ) -> bool {
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
                self.mechanics.profile.underground_sees_surface,
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
                base.walk_destinations.clear();
            }
            base.has_follow_path = false;
            base.force_update_follow_path = false;
            base.follow_target = Some(target_id);
            base.is_updating_path = true;
        }
        let arm_idle = !self.creatures.get(monster_id).is_some_and(|k| {
            matches!(
                k,
                CreatureKind::Monster(m)
                    if m.harness_preserve_sleep
                        && m.state == MonsterState::Sleeping
                        && m.is_idle
            )
        });
        if arm_idle {
            self.request_idle_stimulus(monster_id);
        }
        true
    }
}
