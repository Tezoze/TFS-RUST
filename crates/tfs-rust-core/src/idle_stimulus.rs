//! 772 drain-triggered idle AI — `IdleStimulus` on ToDo queue drain.
//!
//! - `TCreature::IdleStimulus` — virtual dispatch after `Execute` drains the action list.
//! - `TMonster::IdleStimulus` — `crnonpl.cc:2386`.
//!
//! Profile-gated via `GameWorld::beat_driven_loop` (same flag as P2 ToDo walk).

use std::time::Instant;

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use slotmap::Key;
use tfs_rust_common::enums::{CombatType, ConditionType, ZoneType};
use tfs_rust_common::Position;

use crate::chase_debug;
use crate::combat::math::spell_damage;
use crate::combat::{CombatDamage, CombatParams};
use crate::condition::{ActiveCondition, ConditionData};
use crate::creature::{
    monster_weapon_attack_distance, CreatureBase, CreatureKind, MonsterChaseMode, MonsterSpell,
    MonsterState, SpellImpact, SpellShape,
};
use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;
use crate::creature_todo::{trace_creature_todo, CreatureAction, MONSTER_IDLE_WAIT_MS};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::monster_ai::{
    chebyshev, manhattan, monster_idle_chase_step_budget, monster_master_follow_in_wait_band,
    MonsterCombatCloseChaseEnqueue, MonsterEnqueueAttackResult, MonsterIdleChaseRepathOutcome,
};
use crate::monster_targets::TargetSearchType;
use crate::player_flags::{flags_for_group, has_player_flag, PLAYER_FLAG_IGNORED_BY_MONSTERS};

/// C++ `TMonster::IdleStimulus` walking arms — `crnonpl.cc:2676`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonsterIdleWalkBranch {
    /// `crnonpl.cc:2678` — `IsFleeing` + `SearchFlightField`.
    Flee,
    /// `crnonpl.cc:2686` — summon following master.
    MasterFollow,
    /// `crnonpl.cc:2732` — melee `ToDoGo` toward target.
    MeleeChase,
    /// `crnonpl.cc:2751` — adjacent cardinal sidestep.
    MeleeDance,
    /// `crnonpl.cc:2762` — too close for keep-distance band.
    DistFlee,
    /// `crnonpl.cc:2769` — approach keep-distance band.
    DistChase,
    /// `crnonpl.cc:2787` — lateral at keep-distance.
    DistDance,
    /// `crnonpl.cc:2850` — random roam when no target.
    Roam,
    /// At band / no movement this idle tick.
    Hold,
}

/// Result of executing one idle walk arm.
enum MonsterIdleWalkOutcome {
    QueuedGo { via: &'static str, wait_after: bool },
    QueuedWait,
    Noway,
    Hold,
}

/// Which todo action ran — drives post-execute chaining.
pub(crate) enum TodoExecuteKind {
    Go,
    Wait,
    Attack,
    DistanceAttack,
    AttackDeferred,
}

impl GameWorld {
    /// 772 `TCreature::IdleStimulus` — dispatch on creature kind.
    pub(crate) fn idle_stimulus(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        if !self.creatures.contains_key(cid) {
            return;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().todo.locked)
        {
            return;
        }
        match self.creatures.get(cid) {
            Some(CreatureKind::Monster(_)) => {
                trace_creature_todo(self, cid, "idle_stimulus_enter");
                self.monster_idle_stimulus(cid);
                trace_creature_todo(self, cid, "idle_stimulus_exit");
            }
            _ => {}
        }
    }

    /// Request idle when the action queue is drained — sync or deferred to next wakeup.
    pub(crate) fn request_idle_stimulus(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        if !self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
        {
            return;
        }
        if !self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().walk_timer_idle(self.beat_driven_loop))
        {
            return;
        }
        if !self.creature_todo_queue_empty(cid) {
            return;
        }
        if self.creatures.get(cid).is_some_and(|k| {
            matches!(
                k,
                CreatureKind::Monster(m) if m.idle_stimulus_last_ms == Some(self.server_ms)
            )
        }) {
            return;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().todo.has_wait())
        {
            return;
        }
        // C++ `ToDoYield` — schedule `ToDoWait(0)` + `ToDoStart`; `IdleStimulus` runs when
        // the todo list drains on wakeup, not inline from appear/move stimuli (`cract.cc:1001`).
        trace_creature_todo(self, cid, "request_idle_stimulus");
        self.creature_todo_yield(cid);
    }

    /// 772 `EXHAUSTED` recovery — the `TMonster::IdleStimulus` catch block
    /// (`crnonpl.cc:2890-2898`): `Target = 0; ToDoClear(); ToDoWait(1000); ToDoStart();`.
    ///
    /// Invoked when a pre-step kick ([`crate::monster_push`]) hit a player tile or had to kill a
    /// blocker (`KickCreature` returned `false`) — the mover does **not** step this beat, it drops
    /// its target and stalls for a full second instead of clearing-queue and re-planning on the
    /// same beat (audit Finding 7).
    pub(crate) fn monster_exhausted_wait_772(&mut self, cid: CreatureId) {
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.clear_targets();
            base.walk_queue.clear();
            base.has_follow_path = false;
            base.force_update_follow_path = true;
            base.todo.queue.clear();
            base.todo.locked = false;
        }
        trace_creature_todo(self, cid, "monster_exhausted_wait");
        self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
    }

    /// Apply combat damage and fire 772 `DamageStimulus` when a monster loses HP.
    ///
    /// C++ reference: `Game::combatChangeHealth` → `TMonster::DamageStimulus` — `crnonpl.cc:2278`.
    pub(crate) fn combat_execute_with_stimulus(
        &mut self,
        attacker: Option<CreatureId>,
        target: CreatureId,
        damage: &CombatDamage,
        params: &CombatParams,
    ) -> bool {
        let stimulus_damage = (-(damage.primary.1 + damage.secondary.1)).max(0);
        if let Some(attacker_id) = attacker {
            if stimulus_damage > 0 {
                // C++ `DamageStimulus` runs before HP apply — `crmain.cc:631`, `694`.
                self.monster_damage_stimulus(target, attacker_id, stimulus_damage);
            }
        }
        let applied = crate::combat::execute(&mut self.creatures, attacker, target, damage, params);
        if applied {
            let hp_after = self
                .creatures
                .get(target)
                .map(|k| k.base().health)
                .unwrap_or(0);
            if hp_after <= 0 && self.creatures.contains_key(target) {
                self.apply_creature_death(target);
            }
        }
        applied
    }

    /// C++ `TMonster::DamageStimulus` — `crnonpl.cc:2278`.
    pub(crate) fn monster_damage_stimulus(
        &mut self,
        victim_id: CreatureId,
        attacker_id: CreatureId,
        damage: i32,
    ) {
        if !self.beat_driven_loop || damage <= 0 || attacker_id == victim_id {
            return;
        }
        let snapshot = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(victim_id) else {
                return;
            };
            let has_target = m.base.attack_target.is_some() || m.base.follow_target.is_some();
            let old_state = m.state;
            let was_sleeping = old_state == MonsterState::Sleeping;
            let new_state = if was_sleeping {
                if has_target {
                    MonsterState::UnderAttack
                } else {
                    MonsterState::Panic
                }
            } else if !has_target {
                MonsterState::Panic
            } else if old_state == MonsterState::Idle {
                MonsterState::UnderAttack
            } else {
                old_state
            };
            (
                old_state,
                new_state,
                has_target,
                was_sleeping,
                m.base.name.clone(),
            )
        };
        let (old_state, new_state, has_target, was_sleeping, name) = snapshot;
        let state_changed = new_state != old_state;

        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(victim_id) {
            m.state = new_state;
            if new_state == MonsterState::Panic || new_state == MonsterState::UnderAttack {
                m.is_idle = false;
            }
            if !has_target {
                m.opponent_ids.retain(|&id| id != attacker_id);
                if !m.opponent_ids.contains(&attacker_id) {
                    m.opponent_ids.push(attacker_id);
                }
            }
        }

        if chase_debug::chase_path_debug_enabled() {
            chase_debug::log_damage_stimulus(
                self.chase_trace_tick(),
                victim_id,
                name.as_str(),
                Self::monster_state_trace_str(old_state),
                Self::monster_state_trace_str(new_state),
                attacker_id.data().as_ffi(),
                damage,
                has_target,
            );
        }

        if state_changed || was_sleeping {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(victim_id) {
                // First melee after damage lands on the second post-damage idle (`tick=4000` in panic sim).
                m.base.delay_attack_ms(self.server_ms, 4000);
            }
            self.creature_todo_yield(victim_id);
        }
        // C++ `TMonster::DamageStimulus` — state + `ToDoYield` only (`crnonpl.cc:2304`);
        // target pick is idle `Strategy[]`, not synchronous `searchTarget`.
        if !has_target && !self.beat_driven_loop {
            self.monster_try_acquire_chase_target(victim_id, Some(attacker_id));
        }
    }

    fn monster_state_trace_str(state: MonsterState) -> &'static str {
        match state {
            MonsterState::Sleeping => "sleeping",
            MonsterState::Idle => "idle",
            MonsterState::UnderAttack => "under_attack",
            MonsterState::Attacking => "attacking",
            MonsterState::Panic => "panic",
        }
    }

    /// C++ `TMonster::CreatureMoveStimulus` sleep wake — `crnonpl.cc:2866`.
    pub(crate) fn monster_sleep_wake_on_creature_move(
        &mut self,
        monster_id: CreatureId,
        moved_id: CreatureId,
    ) {
        if !self.beat_driven_loop {
            return;
        }
        let sleeping = self.creatures.get(monster_id).is_some_and(
            |k| matches!(k, CreatureKind::Monster(m) if m.state == MonsterState::Sleeping),
        );
        if !sleeping {
            return;
        }
        if moved_id == monster_id {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
                m.state = MonsterState::Idle;
                m.is_idle = false;
            }
            self.add_creature_think_check(monster_id);
            self.creature_todo_yield(monster_id);
            return;
        }
        let should_wake = self.creatures.get(moved_id).is_some_and(|k| match k {
            CreatureKind::Npc(_) => false,
            CreatureKind::Monster(m) => {
                !m.base.is_summon() && m.opponent_ids.is_empty() && !m.is_hostile
            }
            CreatureKind::Player(_) => true,
        });
        if !should_wake {
            return;
        }
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
            m.state = MonsterState::Idle;
            m.is_idle = false;
        }
        self.add_creature_think_check(monster_id);
        self.creature_todo_yield(monster_id);
    }

    /// Roll 772 `Strategy[]` bucket — `crnonpl.cc:2424` (last bucket is random).
    fn monster_idle_roll_strategy_from_roll(
        nearest: u8,
        health: u8,
        damage: u8,
        mut roll: i32,
    ) -> u8 {
        let thresholds = [nearest, health, damage];
        for (idx, &threshold) in thresholds.iter().enumerate() {
            if roll < i32::from(threshold) {
                return idx as u8;
            }
            roll -= i32::from(threshold);
        }
        3
    }

    /// Roll 772 `Strategy[]` bucket — `crnonpl.cc:2424` (last bucket is random).
    fn monster_idle_roll_strategy(nearest: u8, health: u8, damage: u8, rng: &mut impl Rng) -> u8 {
        Self::monster_idle_roll_strategy_from_roll(nearest, health, damage, rng.gen_range(0..100))
    }

    /// C++ target validity + `LoseTarget` — `crnonpl.cc:2368-2384`.
    fn monster_idle_772_lose_existing_target(&mut self, cid: CreatureId) {
        let target_id = self.creatures.get(cid).and_then(|k| k.base().follow_target);
        let Some(target_id) = target_id else {
            return;
        };
        if self.monster_idle_772_should_lose_target(cid, target_id) {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().clear_targets();
            }
        }
    }

    fn monster_idle_772_should_lose_target(&self, cid: CreatureId, target_id: CreatureId) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return true;
        };
        if m.base.master.is_some() {
            return false;
        }
        let Some(target) = self.creatures.get(target_id) else {
            return true;
        };
        let pos = m.base.position;
        let tp = target.position();
        if tp.z != pos.z {
            return true;
        }
        if (pos.x as i32 - tp.x as i32).unsigned_abs() > 10
            || (pos.y as i32 - tp.y as i32).unsigned_abs() > 10
        {
            return true;
        }
        if let Some(tile) = self.map.get_tile(tp) {
            if tile.body().zone == ZoneType::Protection {
                return true;
            }
        }
        // C++ `|| (Master==0 && random(0,99) < LoseTarget)` — draw always when no master
        // (`crnonpl.cc:2381`), even at LoseTarget=0.
        if m.base.master.is_none() {
            let _trace = crate::sim_glibc_rand::sim_rng_trace_site("idle_lose_target");
            let roll = self.parity_random(0, 99);
            if roll < i32::from(m.lose_target_percent) {
                return true;
            }
        }
        false
    }

    /// C++ `TFindCreatures` + `Strategy[]` target pick — `crnonpl.cc:2420-2516`.
    ///
    /// Returns `true` when idle should stop (monster entered sleep).
    fn monster_idle_772_acquire_target(&mut self, cid: CreatureId) -> bool {
        let snapshot = self.creatures.get(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            if m.base.is_summon() || m.base.master.is_some() {
                return None;
            }
            Some((
                m.base.position,
                m.base.follow_target,
                m.state,
                m.strategy_nearest,
                m.strategy_health,
                m.strategy_damage,
            ))
        });
        let Some((pos, existing_follow, state, strat_near, strat_hp, strat_dmg)) = snapshot else {
            return false;
        };

        let has_target = existing_follow.is_some();
        if has_target {
            return false;
        }

        let strategy = Self::monster_idle_roll_strategy_from_roll(
            strat_near,
            strat_hp,
            strat_dmg,
            {
                let _trace = crate::sim_glibc_rand::sim_rng_trace_site("idle_strategy");
                self.parity_random(0, 99)
            },
        );
        let mut should_sleep = true;
        let mut best_param = i32::MIN;
        let mut best_id = None;
        let mut best_tie = 0i32;

        let mut candidates = Vec::new();
        self.map
            .grid
            .collect_spectators(pos.x, pos.y, pos.z, 12, 12, &mut candidates);

        for target_id in &candidates {
            if *target_id == cid {
                continue;
            }
            let Some(target) = self.creatures.get(*target_id) else {
                continue;
            };
            if target.position().z == pos.z {
                should_sleep = false;
            }
            if matches!(target, CreatureKind::Monster(m) if !m.base.is_summon()) {
                continue;
            }
            let tp = target.position();
            if tp.z != pos.z {
                continue;
            }
            let dx = (tp.x as i32 - pos.x as i32).abs();
            let dy = (tp.y as i32 - pos.y as i32).abs();
            if dx > 10 || dy > 10 {
                continue;
            }
            if let Some(tile) = self.map.get_tile(tp) {
                if tile.body().zone == ZoneType::Protection {
                    continue;
                }
            }
            if matches!(target, CreatureKind::Player(p) if {
                let flags = flags_for_group(&self.groups, p.group_id);
                has_player_flag(flags, PLAYER_FLAG_IGNORED_BY_MONSTERS)
            }) {
                continue;
            }
            let param = match strategy {
                0 => -(dx + dy),
                1 => -target.base().health,
                2 => self
                    .creatures
                    .get(cid)
                    .map(|k| k.base().damage_map.get(&target_id).copied().unwrap_or(0) as i32)
                    .unwrap_or(0),
                _ => 0,
            };
            let tie = self.parity_random(0, 99);
            if param > best_param || (param == best_param && tie > best_tie) {
                best_param = param;
                best_tie = tie;
                best_id = Some(target_id);
            }
        }

        if let Some(target_id) = best_id {
            self.monster_add_opponent(cid, *target_id, true);
            let _ = self.monster_select_target(cid, *target_id);
        }

        let state = self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Monster(m) => Some(m.state),
            _ => None,
        });
        let still_no_target = self
            .creatures
            .get(cid)
            .is_none_or(|k| k.base().follow_target.is_none());

        if should_sleep
            && still_no_target
            && !matches!(state, Some(MonsterState::UnderAttack | MonsterState::Panic))
        {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.state = MonsterState::Sleeping;
                m.is_idle = true;
                m.base.clear_targets();
            }
            self.remove_creature_think_check(cid);
            return true;
        }

        if state == Some(MonsterState::Panic) && still_no_target {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.state = MonsterState::Idle;
            }
        }
        if state == Some(MonsterState::UnderAttack) {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.state = MonsterState::Idle;
            }
        }
        false
    }

    /// Resolve cast target — C++ single `Target` field (`follow_target` / `attack_target`).
    fn monster_cast_target_id(base: &CreatureBase) -> Option<CreatureId> {
        base.follow_target.or(base.attack_target)
    }

    /// Tile set for a spell shape — `crnonpl.cc:2627`.
    fn monster_idle_spell_tiles(
        shape: SpellShape,
        caster_pos: Position,
        target_pos: Position,
        radius: i32,
    ) -> Vec<Position> {
        match shape {
            SpellShape::Actor => vec![caster_pos],
            SpellShape::Victim | SpellShape::Destination => vec![target_pos],
            SpellShape::Origin => {
                let mut tiles = vec![caster_pos];
                let r = radius.max(0) as u32;
                for dx in -(r as i32)..=(r as i32) {
                    for dy in -(r as i32)..=(r as i32) {
                        if dx.unsigned_abs().max(dy.unsigned_abs()) <= r {
                            let x = (caster_pos.x as i32 + dx).clamp(0, u16::MAX as i32) as u16;
                            let y = (caster_pos.y as i32 + dy).clamp(0, u16::MAX as i32) as u16;
                            let p = Position::new(x, y, caster_pos.z);
                            if p != caster_pos {
                                tiles.push(p);
                            }
                        }
                    }
                }
                tiles
            }
            SpellShape::Angle => {
                let mut tiles = Vec::new();
                let dx = (target_pos.x as i32 - caster_pos.x as i32).signum();
                let dy = (target_pos.y as i32 - caster_pos.y as i32).signum();
                let steps = radius.max(1) as u32;
                for i in 0..=steps {
                    let x = (caster_pos.x as i32 + dx * i as i32).clamp(0, u16::MAX as i32) as u16;
                    let y = (caster_pos.y as i32 + dy * i as i32).clamp(0, u16::MAX as i32) as u16;
                    tiles.push(Position::new(x, y, caster_pos.z));
                }
                tiles
            }
        }
    }

    /// C++ CASTING block — `crnonpl.cc:2521-2667`.
    fn monster_idle_try_casting(&mut self, cid: CreatureId) {
        let (spells, db_name, cast_target, pos) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.spells.clone(),
                m.base.name.to_ascii_lowercase(),
                Self::monster_cast_target_id(&m.base),
                m.base.position,
            ),
            _ => return,
        };
        let defense_delay_moduli = self
            .monsters_db
            .monsters
            .get(&db_name)
            .map(|mtype| {
                mtype
                    .defenses
                    .spells
                    .iter()
                    .filter_map(MonsterSpell::try_from_node)
                    .filter_map(|spell| (spell.delay > 0).then_some(spell.delay as u32))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if spells.is_empty() && defense_delay_moduli.is_empty() {
            return;
        }
        let Some(target_id) = cast_target else {
            for delay in defense_delay_moduli {
                let _ = self.parity_rand_mod(delay);
            }
            return;
        };
        let target_pos = match self.creatures.get(target_id) {
            Some(k) => k.position(),
            None => return,
        };
        let fleeing = self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.is_fleeing()));

        let mut rng_scratch = StdRng::from_entropy();
        let mut rng_1098 = if self.beat_driven_loop {
            None
        } else {
            Some(std::mem::replace(&mut self.ai_rng, StdRng::from_entropy()))
        };
        for spell in &spells {
            if spell.delay <= 0 || self.parity_rand_mod(spell.delay as u32) != 0 {
                continue;
            }
            if fleeing && self.parity_random(1, 3) != 1 {
                continue;
            }

            let dist = chebyshev(pos, target_pos);
            if spell.range > 0 && dist > spell.range {
                continue;
            }
            if self.monster_idle_suppress_adjacent_melee_spell(cid, dist) {
                continue;
            }

            let tiles = Self::monster_idle_spell_tiles(spell.shape, pos, target_pos, spell.radius);
            let rng: &mut StdRng = rng_1098.as_mut().unwrap_or(&mut rng_scratch);

            match spell.shape {
                SpellShape::Victim | SpellShape::Destination => {
                    if !self.monster_sight_clear(pos, target_pos) {
                        continue;
                    }
                    self.monster_update_look_direction(cid);
                    if let Some(shoot) = spell.shoot_effect {
                        self.broadcast_distance_shoot(pos, target_pos, shoot);
                    }
                    self.monster_idle_apply_spell_impact(cid, target_id, spell, rng);
                }
                SpellShape::Actor => {
                    self.monster_idle_apply_spell_impact(cid, cid, spell, rng);
                }
                SpellShape::Origin | SpellShape::Angle => {
                    for tile in tiles {
                        if !self.monster_sight_clear(pos, tile) {
                            continue;
                        }
                        let victims: Vec<CreatureId> = self
                            .map
                            .get_tile(tile)
                            .map(|t| t.body().creatures.clone())
                            .unwrap_or_default();
                        for victim_id in victims {
                            if victim_id == cid {
                                continue;
                            }
                            if let Some(shoot) = spell.shoot_effect {
                                self.broadcast_distance_shoot(pos, tile, shoot);
                            }
                            self.monster_idle_apply_spell_impact(cid, victim_id, spell, rng);
                        }
                    }
                }
            }

            // C++ CASTING (`crnonpl.cc:2521-2667`) has **no** `break` — every spell whose delay/flee
            // gates pass is evaluated and cast in the same idle, and each spell's delay roll is drawn
            // regardless (audit Finding 2). Stopping after the first cast desyncs the glibc stream.
        }
        if let Some(rng) = rng_1098 {
            self.ai_rng = rng;
        }
        // C++ `RaceData` spell list includes defense entries — consume delay rolls only.
        for delay in defense_delay_moduli {
            let _ = self.parity_rand_mod(delay);
        }
    }

    fn monster_idle_suppress_adjacent_melee_spell(&self, cid: CreatureId, dist: i32) -> bool {
        if !self.beat_driven_loop || dist > 1 {
            return false;
        }
        self.creatures.get(cid).is_some_and(|k| {
            matches!(
                k,
                CreatureKind::Monster(m)
                    if self.monster_effective_target_distance(m.target_distance) <= 1
                        && m.melee_skill > 0
            )
        })
    }

    fn monster_idle_apply_spell_impact(
        &mut self,
        caster_id: CreatureId,
        target_id: CreatureId,
        spell: &MonsterSpell,
        rng: &mut impl Rng,
    ) {
        if chase_debug::chase_path_debug_enabled() {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(caster_id) {
                let spell_label = match &spell.impact {
                    SpellImpact::Damage { .. } => "damage".into(),
                    SpellImpact::Condition { condition, .. } => format!("condition:{condition:?}"),
                    SpellImpact::Healing { .. } => "healing".into(),
                    SpellImpact::Speed { .. } => "speed".into(),
                    SpellImpact::Field => "field".into(),
                    SpellImpact::Summon { race, .. } => format!("summon:{race}"),
                    SpellImpact::Drunk { .. } => "drunk".into(),
                };
                let shape = match spell.shape {
                    SpellShape::Victim => "victim",
                    SpellShape::Actor => "actor",
                    SpellShape::Origin => "origin",
                    SpellShape::Destination => "destination",
                    SpellShape::Angle => "angle",
                };
                chase_debug::log_spell_cast(
                    self.chase_trace_tick(),
                    caster_id,
                    m.base.name.as_str(),
                    &spell_label,
                    target_id.data().as_ffi(),
                    shape,
                    spell.range,
                );
            }
        }
        let profile = self.mechanics.profile;
        let hooks = &self.mechanics.hooks;

        match &spell.impact {
            SpellImpact::Condition {
                condition,
                cycle,
                min_cycle,
            } => {
                let min_c = (*min_cycle).max(1);
                let max_c = (*cycle).max(min_c);
                let strength = if self.beat_driven_loop {
                    self.parity_random(min_c, max_c)
                } else {
                    rng.gen_range(min_c..=max_c)
                };
                let cond = ActiveCondition {
                    id: 0,
                    sub_id: 0,
                    ctype: *condition,
                    data: ConditionData::Damage {
                        total_rank: strength,
                    },
                    timer_rounds_left: None,
                };
                let params = CombatParams {
                    primary_type: CombatType::Physical,
                    dispel: None,
                    apply_condition: Some(cond),
                };
                let _ = self.combat_execute_with_stimulus(
                    Some(caster_id),
                    target_id,
                    &CombatDamage {
                        primary: (CombatType::Physical, 0),
                        secondary: (CombatType::Physical, 0),
                    },
                    &params,
                );
            }
            SpellImpact::Damage {
                element,
                base,
                variation,
            } => {
                let min_dmg = (*base).saturating_sub(*variation);
                let max_dmg = (*base).saturating_add(*variation);
                let scaled = spell_damage(&profile, hooks, 0, 0, max_dmg, false, false);
                let dmg = if scaled > 0 {
                    scaled
                } else if self.beat_driven_loop {
                    // C++ `ComputeDamage` monster path: `Damage + random(-Var, Var)` (`magic.cc:776`)
                    // — glibc parity stream, not `ai_rng` (Finding 14).
                    self.parity_random(min_dmg, max_dmg).max(0)
                } else {
                    crate::combat::uniform_random(rng, min_dmg, max_dmg).max(0)
                };
                let params = CombatParams {
                    primary_type: *element,
                    ..CombatParams::default()
                };
                let _ = self.combat_execute_with_stimulus(
                    Some(caster_id),
                    target_id,
                    &CombatDamage {
                        primary: (*element, -dmg),
                        secondary: (CombatType::Physical, 0),
                    },
                    &params,
                );
            }
            SpellImpact::Healing { base, variation } => {
                let min_heal = (*base).saturating_sub(*variation);
                let max_heal = (*base).saturating_add(*variation);
                let heal = if self.beat_driven_loop {
                    self.parity_random(min_heal, max_heal).max(0)
                } else {
                    crate::combat::uniform_random(rng, min_heal, max_heal).max(0)
                };
                let _ = self.combat_execute_with_stimulus(
                    Some(caster_id),
                    target_id,
                    &CombatDamage {
                        primary: (CombatType::Healing, heal),
                        secondary: (CombatType::Physical, 0),
                    },
                    &CombatParams::default(),
                );
            }
            SpellImpact::Speed {
                percent,
                variation,
                duration: _,
            } => {
                let min_delta = (*percent).saturating_sub(*variation);
                let max_delta = (*percent).saturating_add(*variation);
                let flat_delta = if self.beat_driven_loop {
                    self.parity_random(min_delta, max_delta)
                } else {
                    crate::combat::uniform_random(rng, min_delta, max_delta)
                };
                let cond = ActiveCondition {
                    id: 0,
                    sub_id: 0,
                    ctype: ConditionType::Haste,
                    data: ConditionData::Speed { flat_delta },
                    timer_rounds_left: None,
                };
                let params = CombatParams {
                    primary_type: CombatType::Physical,
                    dispel: None,
                    apply_condition: Some(cond),
                };
                let _ = self.combat_execute_with_stimulus(
                    Some(caster_id),
                    target_id,
                    &CombatDamage {
                        primary: (CombatType::Physical, 0),
                        secondary: (CombatType::Physical, 0),
                    },
                    &params,
                );
            }
            SpellImpact::Drunk { drunkness } => {
                if let Some(kind) = self.creatures.get_mut(target_id) {
                    kind.base_mut().drunkenness = (*drunkness).max(0) as u32;
                }
            }
            SpellImpact::Field => {
                tracing::debug!(
                    caster = ?caster_id,
                    target = ?target_id,
                    "monster spell field impact not yet placed on map"
                );
            }
            SpellImpact::Summon { race, max } => {
                let master_gated = self.creatures.get(caster_id).is_some_and(
                    |k| matches!(k, CreatureKind::Monster(m) if m.base.master.is_none()),
                );
                if master_gated {
                    tracing::debug!(
                        race = %race,
                        max = max,
                        "monster summon spell stub"
                    );
                }
            }
        }
    }

    /// 772 `TMonster::IdleStimulus` — chase/repath/roam decisions (772 only).
    pub(crate) fn monster_idle_stimulus(&mut self, cid: CreatureId) {
        self.monster_idle_stimulus_inner(cid, false);
    }

    /// C++ `CreatureMoveStimulus` may run idle repath in the same beat as a prior `IdleStimulus`
    /// (`crmain.cc:919-961`) — clear per-beat dedup before re-entering.
    pub(crate) fn monster_idle_stimulus_after_creature_move(&mut self, cid: CreatureId) {
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.idle_stimulus_last_ms = None;
        }
        self.monster_idle_stimulus_inner(cid, false);
    }

    /// C++ `TMonster::IdleStimulus` — `crnonpl.cc:2345`.
    ///
    /// When `skip_casting` is true the CASTING block was already executed on this
    /// todo drain pass (`TDAttack` distance tail — `cract.cc:764-767`).
    fn monster_idle_stimulus_inner(&mut self, cid: CreatureId, skip_casting: bool) {
        if !self.creatures.contains_key(cid) {
            return;
        }
        if self.creatures.get(cid).is_some_and(|k| {
            matches!(
                k,
                CreatureKind::Monster(m) if m.idle_stimulus_last_ms == Some(self.server_ms)
            )
        }) {
            return;
        }
        if chase_debug::chase_path_debug_enabled() {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                chase_debug::log_idle_stimulus(self.chase_trace_tick(), cid, &m.base.name);
            }
        }
        if self.beat_driven_loop {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.idle_stimulus_last_ms = Some(self.server_ms);
                // C++ logs `combat_state` each idle pass; harness compare is per-tick bucketed.
                m.last_combat_trace = None;
            }
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.wants_lua_think()))
        {
            return;
        }

        let (is_idle, is_summon, has_opponents, follow, fleeing, pos, sleeping_772) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return;
            };
            (
                m.is_idle,
                m.base.is_summon(),
                !m.opponent_ids.is_empty(),
                m.base.follow_target,
                m.is_fleeing(),
                m.base.position,
                self.beat_driven_loop && m.state == MonsterState::Sleeping,
            )
        };

        if sleeping_772 {
            if is_idle {
                return;
            }
            // Bridge: legacy/test paths may clear `is_idle` before promoting `state` off Sleeping.
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.state = MonsterState::Idle;
            }
        } else if !self.beat_driven_loop && is_idle {
            return;
        }

        if self.beat_driven_loop {
            self.monster_idle_772_lose_existing_target(cid);
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                if !m.is_fleeing() {
                    m.flee_opening_melee_dance_done = false;
                }
            }
            self.monster_idle_reset_combat_state(cid);
            self.monster_idle_try_talk(cid);
            if self.monster_idle_772_acquire_target(cid) {
                return;
            }
            if !skip_casting {
                self.monster_idle_try_casting(cid);
            }
        }

        if is_summon {
            self.monster_think_summon_stub(cid);
        } else if !self.beat_driven_loop && has_opponents {
            if follow.is_none() {
                let _ = self.monster_search_target(cid, TargetSearchType::Default);
            }
            if fleeing {
                let attack = self.creatures.get(cid).and_then(|k| k.base().attack_target);
                if let Some(target_id) = attack {
                    if !self.monster_can_use_attack(cid, pos, target_id) {
                        let _ = self.monster_search_target(cid, TargetSearchType::AttackRange);
                    }
                }
            }
        }

        if !self.beat_driven_loop {
            self.monster_on_think_target(cid, EVENT_CREATURE_THINK_INTERVAL_MS);
            self.monster_update_look_direction(cid);
        }

        if !self
            .creatures
            .get(cid)
            .is_some_and(|k| {
                k.base().health > 0
                    && (k.base().walk_timer_idle(self.beat_driven_loop)
                        || k.base().force_update_follow_path)
            })
        {
            return;
        }

        self.monster_idle_prepare_and_enqueue_go(cid);

        // C++ `Rotate(Target)` after walk arms, before `ToDoAttack` — `crnonpl.cc:2871`.
        self.monster_idle_rotate_toward_attack_target(cid);

        // C++ idle tail appends `ToDoAttack` even when walk already queued `ToDoGo` (`crnonpl.cc:2795`).
        let attack_enqueued = self.monster_idle_maybe_enqueue_attack(cid);
        if self.creature_todo_queue_empty(cid) {
            self.monster_idle_maybe_enqueue_at_goal_wait(cid, attack_enqueued);
        }

        self.monster_idle_reschedule_target_bound_if_parked(cid);
    }

    /// C++ trailing `ToDoStart()` — never leave a live target without a heap wakeup (`crnonpl.cc:2809`).
    fn monster_idle_reschedule_target_bound_if_parked(&mut self, cid: CreatureId) {
        let parked = self.creatures.get(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            let base = k.base();
            let chase_target = base.follow_target.or(base.attack_target)?;
            if !base.todo.is_empty() || base.next_wakeup.is_some() {
                return None;
            }
            if !self.creatures.contains_key(chase_target) {
                return None;
            }
            Some((
                base.name.clone(),
                base.position,
                base.follow_target,
                base.attack_target,
                m.state,
                m.chase_mode,
                chase_target,
            ))
        });
        let Some((name, pos, follow_target, attack_target, state, chase_mode, chase_id)) = parked
        else {
            return;
        };
        if chase_debug::chase_path_debug_enabled() {
            let target_pos = self
                .creatures
                .get(chase_id)
                .map(|k| k.position())
                .unwrap_or(pos);
            let cheb = chebyshev(pos, target_pos);
            let los_clear = self.monster_sight_clear(pos, target_pos);
            let state_str = format!("{state:?}");
            let chase_mode_str = format!("{chase_mode:?}");
            chase_debug::log_parked(
                self.chase_trace_tick(),
                cid,
                name.as_str(),
                pos,
                &state_str,
                follow_target.map(|id| id.data().as_ffi()),
                attack_target.map(|id| id.data().as_ffi()),
                &chase_mode_str,
                cheb,
                los_clear,
            );
        }
        // `ToDoWait(1000)+ToDoStart` fallback when idle arms produced nothing (`crnonpl.cc:2861`).
        self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
    }

    /// C++ `TMonster::IdleStimulus` — `crnonpl.cc:2387` (reset unless PANIC/UNDERATTACK).
    fn monster_idle_reset_combat_state(&mut self, cid: CreatureId) {
        let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) else {
            return;
        };
        if !matches!(m.state, MonsterState::Panic | MonsterState::UnderAttack) {
            m.state = MonsterState::Idle;
        }
    }

    /// C++ talk gate — `crnonpl.cc:2392` (`rand()%50`, then `random(1,Talks)` on hit).
    ///
    /// Sim harness consumes RNG only; no `Talk` packet side effect.
    fn monster_idle_try_talk(&mut self, cid: CreatureId) {
        let talks = self
            .creatures
            .get(cid)
            .and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.talks),
                _ => None,
            })
            .unwrap_or(0);
        if talks == 0 {
            return;
        }
        let _trace_gate = crate::sim_glibc_rand::sim_rng_trace_site("idle_talk_gate");
        if self.parity_rand_mod(50) != 0 {
            return;
        }
        let _trace_pick = crate::sim_glibc_rand::sim_rng_trace_site("idle_talk_pick");
        let _ = self.parity_random(1, i32::from(talks));
    }

    /// C++ walking prelude — `crnonpl.cc:2705` (`SKILL_FIST > 0 && State != PANIC`).
    fn monster_idle_maybe_enter_attacking(&mut self, cid: CreatureId) {
        let should_attack = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return;
            };
            if m.is_fleeing() {
                return;
            }
            let Some(follow_id) = m.base.follow_target else {
                return;
            };
            if m.base.master == Some(follow_id) {
                return;
            }
            if m.state == MonsterState::Panic {
                return;
            }
            m.melee_skill > 0
                && (self.monster_effective_target_distance(m.target_distance) <= 1
                    || !m.spells.iter().any(|s| s.range > 1))
        };
        if should_attack {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                let _entering = m.state != MonsterState::Attacking;
                m.state = MonsterState::Attacking;
                // C++ `SetAttackDest` — chase dest tracks combat target (`crnonpl.cc:2709`).
                if m.base.attack_target.is_none() {
                    if let Some(follow_id) = m.base.follow_target {
                        m.base.attack_target = Some(follow_id);
                    }
                } else if let Some(attack_id) = m.base.attack_target {
                    m.base.follow_target = Some(attack_id);
                }
            }
        }
    }

    /// C++ ATTACKING walk prelude — `crnonpl.cc:2709-2726` (`SetChaseMode` reset then CLOSE for melee).
    pub(crate) fn monster_idle_prepare_combat_chase(&mut self, cid: CreatureId) {
        self.monster_idle_set_combat_chase_mode(cid);
        self.monster_idle_emit_combat_state(cid);
    }

    /// Set `chase_mode` from posture/target band — no JSONL side effect.
    fn monster_idle_set_combat_chase_mode(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        // Keep follow/attack dest aligned for close-chase repath (`SetAttackDest`).
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            if matches!(m.state, MonsterState::Attacking | MonsterState::Panic) {
                if let Some(attack_id) = m.base.attack_target {
                    m.base.follow_target = Some(attack_id);
                }
            }
        }
        let snapshot = self.creatures.get(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            Some((
                m.state,
                m.base.follow_target,
                m.base.position,
                m.target_distance,
            ))
        });
        let Some((state, follow_id, pos, raw_target_distance)) = snapshot else {
            return;
        };
        let new_mode = if !matches!(state, MonsterState::Attacking | MonsterState::Panic) {
            MonsterChaseMode::None
        } else if let Some(follow_id) = follow_id {
            let target_distance = self.monster_effective_target_distance(raw_target_distance);
            let uses_dist_branch =
                self.monster_idle_uses_dist_branch(cid, pos, follow_id, target_distance);
            if uses_dist_branch {
                MonsterChaseMode::None
            } else {
                MonsterChaseMode::Close
            }
        } else {
            MonsterChaseMode::None
        };
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.chase_mode = new_mode;
        }
    }

    /// Emit `combat_state` JSONL when posture/chase_mode changed this idle pass.
    fn monster_idle_emit_combat_state(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        let combat_log = self.creatures.get_mut(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            if !matches!(m.state, MonsterState::Attacking | MonsterState::Panic) {
                m.last_combat_trace = None;
                return None;
            }
            let trace_key = (m.state, m.chase_mode);
            if m.last_combat_trace == Some(trace_key) {
                return None;
            }
            m.last_combat_trace = Some(trace_key);
            let mode = match m.chase_mode {
                MonsterChaseMode::Close => "close",
                MonsterChaseMode::Range => "range",
                MonsterChaseMode::None => "none",
            };
            let state = match m.state {
                MonsterState::Attacking => "attacking",
                MonsterState::Panic => "panic",
                MonsterState::UnderAttack => "under_attack",
                MonsterState::Idle => "idle",
                MonsterState::Sleeping => "sleeping",
            };
            Some((
                m.base.name.clone(),
                state,
                mode,
                m.base.attack_target.map(|id| id.data().as_ffi()),
            ))
        });
        if chase_debug::chase_path_debug_enabled() {
            if let Some((name, state, mode, attack_target)) = combat_log {
                chase_debug::log_combat_state(
                    self.chase_trace_tick(),
                    cid,
                    name.as_str(),
                    state,
                    mode,
                    attack_target,
                );
            }
        }
    }

    /// C++ `TCreature::ToDoAttack` action list — `cract.cc:1325-1334`.
    pub(crate) fn monster_enqueue_todo_attack_actions(
        &mut self,
        cid: CreatureId,
    ) -> MonsterEnqueueAttackResult {
        let (weapon_distance, needs_close_step) = self
            .creatures
            .get(cid)
            .map(|k| match k {
                CreatureKind::Monster(m) => {
                    let weapon_distance = monster_weapon_attack_distance(
                        m.melee_skill,
                        m.spells.iter().any(|s| s.range > 1),
                    );
                    let needs_close_step = m.base.attack_target.is_some_and(|aid| {
                        self.creatures.get(aid).is_some_and(|t| {
                            weapon_distance == 1 && chebyshev(m.base.position, t.position()) > 1
                        })
                    });
                    (weapon_distance, needs_close_step)
                }
                _ => (1, false),
            })
            .unwrap_or((1, false));
        let skip_idle_melee_chase = self.monster_idle_skip_idle_melee_chase(cid);
        let already_has_close_go = self.monster_close_chase_go_already_armed(cid);
        let close_chase = if already_has_close_go {
            MonsterCombatCloseChaseEnqueue::Skipped
        } else {
            self.monster_combat_enqueue_close_chase_go(cid)
        };
        if close_chase == MonsterCombatCloseChaseEnqueue::Retry {
            return MonsterEnqueueAttackResult::Retry;
        }
        if close_chase == MonsterCombatCloseChaseEnqueue::Noway {
            return MonsterEnqueueAttackResult::Noway;
        }
        if needs_close_step
            && close_chase != MonsterCombatCloseChaseEnqueue::Queued
            && !already_has_close_go
            && !skip_idle_melee_chase
        {
            return MonsterEnqueueAttackResult::Failed;
        }
        if weapon_distance != 1 {
            self.enqueue_creature_wait(cid, 100);
        }
        let close_label = if already_has_close_go || skip_idle_melee_chase {
            "idle_tail"
        } else {
            match close_chase {
                MonsterCombatCloseChaseEnqueue::Queued => "queued",
                MonsterCombatCloseChaseEnqueue::Skipped => "skipped",
                MonsterCombatCloseChaseEnqueue::Retry => "retry",
                MonsterCombatCloseChaseEnqueue::Noway => "noway",
            }
        };
        if self.enqueue_creature_attack(cid) {
            if chase_debug::chase_path_debug_enabled() {
                if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                    let wait_ms = if weapon_distance != 1 { 100 } else { 0 };
                    chase_debug::log_attack_enqueue(
                        self.chase_trace_tick(),
                        cid,
                        m.base.name.as_str(),
                        wait_ms,
                        needs_close_step && !already_has_close_go && !skip_idle_melee_chase,
                        close_label,
                    );
                }
            }
            let needs_wakeup = self
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().next_wakeup.is_none() && !k.base().todo.has_go());
            if needs_wakeup {
                let delay_ms = self.todo_attack_delay_ms(cid);
                self.todo_start_from_action(cid, delay_ms);
            }
            MonsterEnqueueAttackResult::Enqueued
        } else {
            MonsterEnqueueAttackResult::Failed
        }
    }

    /// 772 melee tail uses cheb band for strike; enqueue uses `ToDoAttack` walk path (`cract.cc:1325`).
    fn monster_idle_can_enqueue_attack(
        &self,
        cid: CreatureId,
        pos: Position,
        attack_id: CreatureId,
        target_pos: Position,
    ) -> bool {
        if !self.beat_driven_loop {
            return self.monster_can_use_attack(cid, pos, attack_id);
        }
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return false;
        };
        let target_distance = self.monster_effective_target_distance(m.target_distance);
        let dist = chebyshev(pos, target_pos);
        // `CanToDoAttack` close walk at cheb>1 — no strike-range cap (`crcombat.cc:496`).
        if m.melee_skill > 0 && m.chase_mode == MonsterChaseMode::Close {
            return true;
        }
        if target_distance <= 1 {
            return true;
        }
        self.monster_can_use_attack(cid, pos, attack_id)
    }

    /// C++ `Rotate(Target)` at idle combat tail — `crnonpl.cc:2871` (after `ToDoGo`, before `ToDoAttack`).
    pub(crate) fn monster_idle_rotate_toward_attack_target(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        let should_rotate = self.creatures.get(cid).is_some_and(|k| {
            let CreatureKind::Monster(m) = k else {
                return false;
            };
            matches!(m.state, MonsterState::Attacking | MonsterState::Panic)
                && m.base.attack_target.is_some()
        });
        if should_rotate {
            self.monster_update_look_direction(cid);
        }
    }

    /// C++ idle combat tail — `Rotate` + `ToDoAttack` (`crnonpl.cc:2795`).
    fn monster_idle_maybe_enqueue_attack(&mut self, cid: CreatureId) -> bool {
        let (attack_id, pos) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                if m.is_fleeing() {
                    return false;
                }
                let Some(attack_id) = m.base.attack_target else {
                    return false;
                };
                (attack_id, m.base.position)
            }
            _ => return false,
        };
        if !self.creatures.contains_key(attack_id) {
            return false;
        }
        let target_pos = match self.creatures.get(attack_id) {
            Some(k) => k.position(),
            None => return false,
        };
        if !self.monster_idle_can_enqueue_attack(cid, pos, attack_id, target_pos) {
            return false;
        }
        // C++ `CanToDoAttack` close walk does not require LOS — only strike does (`crcombat.cc:496`).
        // C++ always appends `ToDoAttack` at the idle tail (`crnonpl.cc:2795`); cadence is enforced
        // by `TDAttack` on execute (`cract.cc:909`), not by skipping enqueue here.
        match self.monster_enqueue_todo_attack_actions(cid) {
            MonsterEnqueueAttackResult::Enqueued => {
                trace_creature_todo(self, cid, "idle_enqueue_attack");
                true
            }
            MonsterEnqueueAttackResult::Retry => {
                self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
                false
            }
            MonsterEnqueueAttackResult::Noway => {
                self.monster_idle_prepare_and_enqueue_go(cid);
                false
            }
            MonsterEnqueueAttackResult::Failed => {
                self.monster_combat_handle_close_chase_blocked(cid);
                false
            }
        }
    }

    /// Yield and retry close-chase when still off-band; short wait at strike range (`cract.cc:845-852`).
    pub(crate) fn monster_combat_handle_close_chase_blocked(&mut self, cid: CreatureId) {
        let still_off_band = self.creatures.get(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            let attack_id = m.base.attack_target?;
            let target_pos = self.creatures.get(attack_id)?.position();
            Some(chebyshev(m.base.position, target_pos) > 1)
        });
        if still_off_band == Some(true) {
            self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
        } else {
            self.idle_enqueue_wait_and_start(cid, 200);
        }
    }

    /// `ToDoWait(1000)` when at-goal dance could not arm (`crnonpl.cc:2791` dist band).
    /// Melee `ATTACKING` tail gets `ToDoAttack` only — no trailing wait (`crnonpl.cc:2795–2807`).
    fn monster_idle_maybe_enqueue_at_goal_wait(&mut self, cid: CreatureId, attack_enqueued: bool) {
        if !self.beat_driven_loop {
            return;
        }
        let branch = self.monster_idle_classify_walk_branch(cid);
        match branch {
            MonsterIdleWalkBranch::DistDance => {}
            MonsterIdleWalkBranch::MeleeDance => {
                if attack_enqueued {
                    return;
                }
                if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| k.base().todo.has_attack())
                {
                    return;
                }
                let hostile_melee_at_band = match self.creatures.get(cid) {
                    Some(CreatureKind::Monster(m)) => match m.base.follow_target {
                        None => false,
                        Some(follow_id) => {
                            let target_distance =
                                self.monster_effective_target_distance(m.target_distance);
                            if target_distance > 1 {
                                false
                            } else if let Some(t) = self.creatures.get(follow_id) {
                                chebyshev(m.base.position, t.position()) == 1
                                    && self.monster_idle_is_attacking_posture(cid, target_distance)
                            } else {
                                false
                            }
                        }
                    },
                    _ => false,
                };
                if hostile_melee_at_band {
                    return;
                }
            }
            _ => return,
        }
        self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
    }

    /// When idle should run [`GameWorld::monster_idle_chase_repath`] for an active chase (772 only).
    pub(crate) fn monster_idle_chase_needs_repath(
        &mut self,
        cid: CreatureId,
    ) -> (bool, Option<&'static str>) {
        let Some(k) = self.creatures.get(cid) else {
            return (false, None);
        };
        let base = k.base();
        if base.force_update_follow_path {
            if let Some(follow_id) = base.follow_target {
                let pos = k.position();
                if let Some(target_pos) = self.creatures.get(follow_id).map(|t| t.position()) {
                    let (fleeing, target_distance) = match self.creatures.get(cid) {
                        Some(CreatureKind::Monster(m)) => (
                            m.is_fleeing(),
                            self.monster_effective_target_distance(m.target_distance),
                        ),
                        _ => return (true, Some("force_update")),
                    };
                    if self.monster_at_follow_goal(
                        cid,
                        follow_id,
                        pos,
                        target_pos,
                        fleeing,
                        target_distance,
                    ) {
                        if let Some(k) = self.creatures.get_mut(cid) {
                            k.base_mut().force_update_follow_path = false;
                        }
                        return (false, None);
                    }
                }
            }
            return (true, Some("force_update"));
        }
        if !base.walk_queue.is_empty() {
            return (false, None);
        }
        if !base.has_follow_path {
            return (true, Some("idle_drain"));
        }
        let Some(follow_id) = base.follow_target else {
            return (false, None);
        };
        let pos = k.position();
        let Some(target_pos) = self.creatures.get(follow_id).map(|t| t.position()) else {
            return (false, None);
        };
        let (fleeing, target_distance) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.is_fleeing(),
                self.monster_effective_target_distance(m.target_distance),
            ),
            _ => return (false, None),
        };
        if self.monster_at_follow_goal(cid, follow_id, pos, target_pos, fleeing, target_distance) {
            return (false, None);
        }
        (true, Some("off_band"))
    }

    /// Classify the idle walk arm — `crnonpl.cc:2676` priority order.
    ///
    /// Melee vs ranged split mirrors `!DistanceFighting || !ThrowPossible` (`crnonpl.cc:2795-2797`)
    /// via [`GameWorld::monster_idle_uses_dist_branch`].
    pub(crate) fn monster_idle_classify_walk_branch(
        &self,
        cid: CreatureId,
    ) -> MonsterIdleWalkBranch {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return MonsterIdleWalkBranch::Hold;
        };

        let follow_id = match m.base.follow_target {
            Some(id) => id,
            None => return MonsterIdleWalkBranch::Roam,
        };

        let pos = m.base.position;
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return MonsterIdleWalkBranch::Roam,
        };
        let target_distance = self.monster_effective_target_distance(m.target_distance);
        let dist = chebyshev(pos, target_pos);

        // X3 — adjacent low-HP flee still dances once before the flee arm (`crnonpl.cc` idle).
        if m.is_fleeing() {
            if dist == 1 && target_distance <= 1 && !m.flee_opening_melee_dance_done {
                return MonsterIdleWalkBranch::MeleeDance;
            }
            return MonsterIdleWalkBranch::Flee;
        }

        if m.base.master == Some(follow_id) {
            return MonsterIdleWalkBranch::MasterFollow;
        }

        let uses_dist_branch =
            self.monster_idle_uses_dist_branch(cid, pos, follow_id, target_distance);

        if uses_dist_branch {
            if dist < target_distance {
                MonsterIdleWalkBranch::DistFlee
            } else if dist > target_distance {
                MonsterIdleWalkBranch::DistChase
            } else {
                MonsterIdleWalkBranch::DistDance
            }
        } else if dist > 1 {
            if self.monster_idle_skip_idle_melee_chase(cid) {
                MonsterIdleWalkBranch::Hold
            } else {
                MonsterIdleWalkBranch::MeleeChase
            }
        } else if dist == 1 {
            MonsterIdleWalkBranch::MeleeDance
        } else {
            MonsterIdleWalkBranch::Hold
        }
    }

    fn monster_idle_log_walk_branch(
        &self,
        cid: CreatureId,
        branch: &str,
        dest: Position,
        must: bool,
        max_steps: i32,
        reason: Option<&str>,
    ) {
        if !chase_debug::chase_path_debug_enabled() {
            return;
        }
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return;
        };
        chase_debug::log_branch(
            self.chase_trace_tick(),
            cid,
            m.base.name.as_str(),
            branch,
            m.base.position,
            dest,
            must,
            max_steps,
            reason,
        );
    }

    /// Execute one classified walk arm — returns outcome without enqueuing `Go`.
    fn monster_idle_execute_walk_branch(
        &mut self,
        cid: CreatureId,
        branch: MonsterIdleWalkBranch,
    ) -> MonsterIdleWalkOutcome {
        match branch {
            MonsterIdleWalkBranch::Flee => {
                if self.monster_idle_flee_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "idle_flee",
                        wait_after: false,
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::DistFlee => {
                if self.monster_idle_flee_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "idle_flee",
                        wait_after: false,
                    }
                } else {
                    MonsterIdleWalkOutcome::QueuedWait
                }
            }
            MonsterIdleWalkBranch::MasterFollow => {
                let (needs_repath, repath_reason) = self.monster_idle_chase_needs_repath(cid);
                if !needs_repath {
                    return self.monster_idle_master_follow_hold_or_wait(cid);
                }
                match self.monster_idle_master_follow(cid, repath_reason) {
                    MonsterIdleChaseRepathOutcome::PathQueued => MonsterIdleWalkOutcome::QueuedGo {
                        via: repath_reason.unwrap_or("idle_drain"),
                        wait_after: false,
                    },
                    MonsterIdleChaseRepathOutcome::AtGoal => {
                        self.monster_idle_master_follow_hold_or_wait(cid)
                    }
                    MonsterIdleChaseRepathOutcome::Noway => MonsterIdleWalkOutcome::Noway,
                }
            }
            MonsterIdleWalkBranch::MeleeChase | MonsterIdleWalkBranch::DistChase => {
                let (needs_repath, repath_reason) = self.monster_idle_chase_needs_repath(cid);
                if !needs_repath {
                    return MonsterIdleWalkOutcome::Hold;
                }
                let branch_name = if branch == MonsterIdleWalkBranch::MeleeChase {
                    "melee_chase"
                } else {
                    "dist_chase"
                };
                let cheb = self
                    .creatures
                    .get(cid)
                    .and_then(|k| {
                        let follow_id = k.base().follow_target?;
                        let target_pos = self.creatures.get(follow_id)?.position();
                        Some(chebyshev(k.position(), target_pos))
                    })
                    .unwrap_or(0);
                let is_melee_chase = branch == MonsterIdleWalkBranch::MeleeChase;
                let is_dist_chase = branch == MonsterIdleWalkBranch::DistChase;
                let target_distance = self
                    .creatures
                    .get(cid)
                    .map(|k| match k {
                        CreatureKind::Monster(m) => {
                            self.monster_effective_target_distance(m.target_distance)
                        }
                        _ => 1,
                    })
                    .unwrap_or(1);
                let (max_steps, must_reach) = monster_idle_chase_step_budget(
                    is_melee_chase,
                    is_dist_chase,
                    cheb,
                    target_distance,
                );
                if let Some(target_pos) = self
                    .creatures
                    .get(cid)
                    .and_then(|k| k.base().follow_target)
                    .and_then(|tid| self.creatures.get(tid).map(|t| t.position()))
                {
                    self.monster_idle_log_walk_branch(
                        cid,
                        branch_name,
                        target_pos,
                        must_reach,
                        max_steps as i32,
                        repath_reason,
                    );
                }
                match self.monster_idle_chase_repath(cid, repath_reason, max_steps, must_reach) {
                    MonsterIdleChaseRepathOutcome::PathQueued => MonsterIdleWalkOutcome::QueuedGo {
                        via: repath_reason.unwrap_or("idle_drain"),
                        wait_after: false,
                    },
                    MonsterIdleChaseRepathOutcome::AtGoal => MonsterIdleWalkOutcome::Hold,
                    MonsterIdleChaseRepathOutcome::Noway => MonsterIdleWalkOutcome::Noway,
                }
            }
            MonsterIdleWalkBranch::MeleeDance => {
                if self.monster_idle_dance_step(cid) {
                    let queued = self.creatures.get(cid).is_some_and(|k| {
                        !k.base().walk_queue.is_empty()
                    });
                    if queued {
                        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                            if m.is_fleeing() {
                                m.flee_opening_melee_dance_done = true;
                            }
                        }
                        MonsterIdleWalkOutcome::QueuedGo {
                            via: "idle_dance",
                            wait_after: false,
                        }
                    } else {
                        // C++ `rand()%5` hold — branch may log but no `ToDoGo` (`crnonpl.cc:2814`).
                        MonsterIdleWalkOutcome::Hold
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::DistDance => {
                if self.monster_idle_dance_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "idle_dance",
                        wait_after: true,
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::Roam => {
                let pos = self
                    .creatures
                    .get(cid)
                    .map(|k| k.position())
                    .unwrap_or(Position::new(0, 0, 7));
                self.monster_idle_log_walk_branch(cid, "roam", pos, false, 1, None);
                if self.monster_idle_roam_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "roam",
                        wait_after: true,
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::Hold => MonsterIdleWalkOutcome::Hold,
        }
    }

    /// Master follow Manhattan 2–3 → `ToDoWait` only (`crnonpl.cc:2691`).
    fn monster_idle_master_follow_hold_or_wait(&self, cid: CreatureId) -> MonsterIdleWalkOutcome {
        let (pos, follow_id) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                let Some(follow_id) = m.base.follow_target else {
                    return MonsterIdleWalkOutcome::Hold;
                };
                (m.base.position, follow_id)
            }
            _ => return MonsterIdleWalkOutcome::Hold,
        };
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return MonsterIdleWalkOutcome::Hold,
        };
        if monster_master_follow_in_wait_band(manhattan(pos, target_pos)) {
            MonsterIdleWalkOutcome::QueuedWait
        } else {
            MonsterIdleWalkOutcome::Hold
        }
    }

    /// Fill walk queue from reference-ordered idle arms, then enqueue `Go` + heap arm.
    ///
    /// C++ walking section — `crnonpl.cc:2676`.
    fn monster_idle_prepare_and_enqueue_go(&mut self, cid: CreatureId) {
        if self.beat_driven_loop {
            self.monster_idle_maybe_enter_attacking(cid);
            self.monster_idle_set_combat_chase_mode(cid);
        }
        let branch = self.monster_idle_classify_walk_branch(cid);
        let mut outcome = self.monster_idle_execute_walk_branch(cid, branch);

        if matches!(outcome, MonsterIdleWalkOutcome::Noway) {
            self.monster_on_chase_noway_772(cid);
            outcome = self.monster_idle_execute_walk_branch(cid, MonsterIdleWalkBranch::Roam);
        }

        if self.beat_driven_loop {
            // C++ logs `combat_state` after PANIC melee-dance promotion (`crnonpl.cc:2830`).
            self.monster_idle_emit_combat_state(cid);
        }

        match outcome {
            MonsterIdleWalkOutcome::QueuedGo { via, wait_after } => {
                self.idle_enqueue_paced_go(
                    cid,
                    true,
                    Some(via),
                    wait_after.then_some(MONSTER_IDLE_WAIT_MS),
                );
            }
            MonsterIdleWalkOutcome::QueuedWait => {
                self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
            }
            MonsterIdleWalkOutcome::Hold => {
                // 772 idle drain owns dance pacing — no TFS `getNextStep` poll (X5).
                if !self.beat_driven_loop && self.monster_should_keep_dance_walk_alive(cid) {
                    self.idle_enqueue_go_and_start(cid, true, None);
                }
            }
            MonsterIdleWalkOutcome::Noway => {}
        }
    }

    /// Execute the front todo action for 772 monsters.
    pub(crate) fn execute_creature_todo_action(
        &mut self,
        cid: CreatureId,
    ) -> Option<TodoExecuteKind> {
        /// Post-unlock idle work — `idle_stimulus` must not run while `todo.locked`.
        enum CombatExecuteFollowUp {
            None,
            IdleStimulus,
            CloseChaseBlocked,
        }

        let action = {
            let Some(k) = self.creatures.get_mut(cid) else {
                return None;
            };
            if k.base().todo.locked {
                return None;
            }
            k.base_mut().todo.queue.pop_front()
        };
        let action = action?;

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().todo.locked = true;
        }

        let mut follow_up = CombatExecuteFollowUp::None;
        let kind = match action {
            CreatureAction::Go => {
                trace_creature_todo(self, cid, "execute_go");
                let now = Instant::now();
                self.on_walk(cid, false, now, None);
                trace_creature_todo(self, cid, "execute_go_done");
                TodoExecuteKind::Go
            }
            CreatureAction::Wait { delay_ms } => {
                trace_creature_todo(self, cid, "execute_wait");
                // C++ chase trace logs `ToDoWait` enqueue only — not execute drain.
                if delay_ms > 0 {
                    self.todo_start_from_action(cid, delay_ms);
                }
                trace_creature_todo(self, cid, "execute_wait_done");
                TodoExecuteKind::Wait
            }
            CreatureAction::Attack => {
                let delay = self.todo_attack_delay_ms(cid);
                if delay > 0 {
                    if let Some(k) = self.creatures.get_mut(cid) {
                        k.base_mut().todo.queue.push_front(CreatureAction::Attack);
                    }
                    trace_creature_todo(self, cid, "execute_attack_deferred");
                    self.todo_start_from_action(cid, delay);
                    trace_creature_todo(self, cid, "execute_attack_deferred_done");
                    TodoExecuteKind::AttackDeferred
                } else {
                    let needs_close_step = self
                        .creatures
                        .get(cid)
                        .and_then(|k| {
                            let CreatureKind::Monster(m) = k else {
                                return None;
                            };
                            let aid = m.base.attack_target?;
                            let cheb =
                                chebyshev(m.base.position, self.creatures.get(aid)?.position());
                            let weapon_dist = monster_weapon_attack_distance(
                                m.melee_skill,
                                m.spells.iter().any(|s| s.range > 1),
                            );
                            Some(weapon_dist == 1 && cheb > 1)
                        })
                        .unwrap_or(false);
                    if needs_close_step {
                        if let Some(k) = self.creatures.get_mut(cid) {
                            k.base_mut().todo.queue.push_front(CreatureAction::Attack);
                        }
                        if self
                            .creatures
                            .get(cid)
                            .is_some_and(|k| k.base().todo.has_go())
                        {
                            trace_creature_todo(self, cid, "execute_attack_wait_for_go");
                            TodoExecuteKind::AttackDeferred
                        } else {
                            match self.monster_combat_enqueue_close_chase_go(cid) {
                                MonsterCombatCloseChaseEnqueue::Queued => {
                                    if self
                                        .creatures
                                        .get(cid)
                                        .is_some_and(|k| k.base().todo.has_go())
                                    {
                                        if self.todo_start_go_delay(cid, false) {
                                            self.schedule_immediate_todo_wakeup(cid);
                                        } else if self
                                            .creatures
                                            .get(cid)
                                            .is_some_and(|k| k.base().next_wakeup.is_none())
                                        {
                                            let _ = self.todo_start_go_delay(cid, false);
                                        }
                                    }
                                }
                                MonsterCombatCloseChaseEnqueue::Retry => {
                                    if let Some(k) = self.creatures.get_mut(cid) {
                                        k.base_mut().todo.queue.pop_front();
                                    }
                                    self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
                                }
                                MonsterCombatCloseChaseEnqueue::Noway => {
                                    if let Some(k) = self.creatures.get_mut(cid) {
                                        k.base_mut().todo.queue.pop_front();
                                    }
                                    follow_up = CombatExecuteFollowUp::IdleStimulus;
                                }
                                MonsterCombatCloseChaseEnqueue::Skipped => {
                                    if let Some(k) = self.creatures.get_mut(cid) {
                                        k.base_mut().todo.queue.pop_front();
                                    }
                                    follow_up = CombatExecuteFollowUp::CloseChaseBlocked;
                                }
                            }
                            trace_creature_todo(self, cid, "execute_attack_out_of_range");
                            TodoExecuteKind::AttackDeferred
                        }
                    } else {
                        let distance_fighter = self.creatures.get(cid).is_some_and(|k| {
                            matches!(
                                k,
                                CreatureKind::Monster(m)
                                    if self.monster_effective_target_distance(m.target_distance) > 1
                            )
                        });
                        trace_creature_todo(self, cid, "execute_attack");
                        self.monster_do_attacking(cid, EVENT_CREATURE_THINK_INTERVAL_MS);
                        if distance_fighter {
                            if let Some(wakeup) = self
                                .creatures
                                .get(cid)
                                .map(|k| k.base().earliest_attack_ms)
                                .filter(|&wakeup| wakeup > self.server_ms)
                            {
                                self.schedule_creature_wakeup(cid, wakeup);
                            }
                        }
                        trace_creature_todo(self, cid, "execute_attack_done");
                        if distance_fighter {
                            TodoExecuteKind::DistanceAttack
                        } else {
                            TodoExecuteKind::Attack
                        }
                    }
                }
            }
        };

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().todo.locked = false;
        }

        match follow_up {
            CombatExecuteFollowUp::IdleStimulus => self.monster_idle_stimulus(cid),
            CombatExecuteFollowUp::CloseChaseBlocked => {
                self.monster_combat_handle_close_chase_blocked(cid);
            }
            CombatExecuteFollowUp::None => {}
        }

        Some(kind)
    }

    /// Execute one `CreatureAction::Go` for 772 monsters — returns true if an action ran.
    pub(crate) fn execute_creature_todo_go(&mut self, cid: CreatureId) -> bool {
        matches!(
            self.execute_creature_todo_action(cid),
            Some(TodoExecuteKind::Go)
        )
    }

    /// After Go/Attack execute: schedule next step or chain queued actions.
    pub(crate) fn finish_creature_todo_execute(&mut self, cid: CreatureId) {
        if !self.creature_uses_todo_execute(cid) {
            return;
        }

        let walk_queue_has_more = self
            .creatures
            .get(cid)
            .is_some_and(|k| !k.base().walk_queue.is_empty());

        if walk_queue_has_more {
            let force_repath = self
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().force_update_follow_path);
            if force_repath {
                if let Some(k) = self.creatures.get_mut(cid) {
                    let base = k.base_mut();
                    base.walk_queue.clear();
                    base.has_follow_path = false;
                }
                self.request_idle_stimulus(cid);
                return;
            }
            // Re-arm `Go` before pending `Attack` — one step per execute (`cract.cc:728`).
            let _ = self.enqueue_creature_go_at(cid, true);
            if self.todo_start_go_delay(cid, false) {
                self.schedule_immediate_todo_wakeup(cid);
            }
            return;
        }

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().has_follow_path = false;
        }

        if !self.creature_todo_queue_empty(cid) {
            // C++ `ToDoGo` completes before chained `TDAttack` when target kited away (`crmain.cc:950`).
            let defer_attack_after_go = self.creatures.get(cid).is_some_and(|k| {
                let CreatureKind::Monster(m) = k else {
                    return false;
                };
                if !m.base.todo.has_attack() || m.base.todo.has_go() {
                    return false;
                }
                if !self.monster_idle_skip_idle_melee_chase(cid) {
                    return false;
                }
                m.base.attack_target.is_some_and(|aid| {
                    self.creatures
                        .get(aid)
                        .is_some_and(|t| chebyshev(k.position(), t.position()) > 1)
                })
            });
            if defer_attack_after_go {
                if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                    m.base.next_wakeup = None;
                }
                let mut delay_ms = self.todo_attack_delay_ms(cid);
                if delay_ms == 0 {
                    delay_ms = 200;
                }
                self.todo_start_from_action(cid, delay_ms);
                return;
            }
            self.run_monster_todo_execute(cid);
            return;
        }

        self.maybe_idle_stimulus_after_go_complete(cid);
    }

    /// Gate harness idle re-entry after todo drain — shared by [`finish_creature_todo_execute`]
    /// and [`GameWorld::process_creature_todo`].
    pub(crate) fn maybe_idle_stimulus_after_go_complete(&mut self, cid: CreatureId) {
        self.monster_idle_stimulus(cid);
    }

    /// Run one queued action (772 monsters).
    pub(crate) fn run_monster_todo_execute(&mut self, cid: CreatureId) {
        match self.execute_creature_todo_action(cid) {
            Some(TodoExecuteKind::Go) | Some(TodoExecuteKind::Attack) => {
                self.finish_creature_todo_execute(cid);
            }
            Some(TodoExecuteKind::DistanceAttack) => {
                self.monster_idle_try_casting(cid);
                if self.creature_todo_queue_empty(cid) {
                    // Future attack cadence lives in `earliest_attack_ms`; do not block the
                    // post-`TDAttack` idle walk arm (`cract.cc:764-767`, `crnonpl.cc:2741`).
                    if let Some(k) = self.creatures.get_mut(cid) {
                        let base = k.base_mut();
                        if base.next_wakeup.is_some_and(|w| w > self.server_ms) {
                            base.next_wakeup = None;
                        }
                    }
                    self.monster_idle_stimulus_inner(cid, true);
                    self.monster_idle_reschedule_target_bound_if_parked(cid);
                } else {
                    self.finish_creature_todo_execute(cid);
                }
            }
            Some(TodoExecuteKind::Wait) => {
                // C++ `TCreature::Execute` — drained todo list runs `IdleStimulus`
                // (`cract.cc:764-767`), including after `ToDoYield`'s `ToDoWait(0)`.
                if self.creature_todo_queue_empty(cid) {
                    self.idle_stimulus(cid);
                    if !self.creature_todo_queue_empty(cid) {
                        self.run_monster_todo_execute(cid);
                    }
                } else {
                    self.monster_combat_reschedule_if_stalled(cid);
                }
            }
            Some(TodoExecuteKind::AttackDeferred) => {
                self.monster_combat_reschedule_if_stalled(cid);
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::enums::{CombatType, ConditionType, Direction};
    use tfs_rust_common::Position;

    use crate::combat::{CombatDamage, CombatParams};
    use crate::creature::{
        CreatureKind, MonsterAiConfig, MonsterChaseMode, MonsterSpell, MonsterState, SpellImpact,
        SpellShape,
    };
    use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;
    use crate::creature_todo::{CreatureAction, MONSTER_IDLE_WAIT_MS};
    use crate::game_world::GameWorld;
    use crate::idle_stimulus::MonsterIdleWalkBranch;
    use crate::ids::CreatureId;
    use crate::monster_ai::{MonsterCombatCloseChaseEnqueue, MonsterEnqueueAttackResult};
    use crate::test_world::support::{
        dist_idle_monster_config, beat_driven_test_world, ensure_walkable_tile,
        insert_monster, insert_monster_with_config, insert_player, minimal_world,
        test_player, TEST_SYNTHETIC_GROUND_WP,
    };

    /// Same-floor creature outside the 10-tile targeting box — `CanSeeFloor` awake without a target.
    fn register_distant_floor_spectator(world: &mut GameWorld, near: Position) -> CreatureId {
        let far = Position::new(near.x.saturating_add(15), near.y, near.z);
        ensure_walkable_tile(&mut world.map, far, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(world, test_player("Spectator", far));
        world.map.register_creature_at(far, player);
        player
    }

    /// Phase A — idle enqueues Go on drain; think no longer arms walk on 772.
    #[test]
    fn idle_stimulus_enqueues_go_for_active_monster() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for x in 101..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.attack_target = Some(player);
        }
        world.add_creature_think_check(monster);
        assert!(
            world.monster_set_follow_creature(monster, Some(player)),
            "set_follow must succeed in view"
        );

        let has_go = world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().todo.has_go());
        let armed = world
            .creatures
            .get(monster)
            .and_then(|k| k.base().next_wakeup)
            .is_some();
        assert!(
            has_go || armed,
            "772 set_follow must enqueue Go or schedule wakeup via idle"
        );

        if world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().todo.has_go())
        {
            world.execute_creature_todo_go(monster);
        }

        world.monster_native_on_think(monster, EVENT_CREATURE_THINK_INTERVAL_MS);
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go()),
            "772 think must not enqueue Go actions"
        );
    }

    /// Phase A — duplicate Go / heap entries suppressed when wakeup already armed.
    #[test]
    fn idle_go_enqueue_respects_wakeup_gate() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        assert!(world.enqueue_creature_go(monster));
        world.todo_start_from_action(monster, 500);
        let wakeup = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .next_wakeup
            .expect("wakeup armed");
        let heap_len = world.todo_queue.len();

        assert!(!world.enqueue_creature_go(monster), "duplicate Go rejected");
        world.request_idle_stimulus(monster);

        assert_eq!(
            world.creatures.get(monster).unwrap().base().next_wakeup,
            Some(wakeup)
        );
        assert_eq!(world.todo_queue.len(), heap_len);
    }

    /// Phase A — process_creature_todo runs idle when action queue empty on wakeup.
    #[test]
    fn process_creature_todo_runs_idle_on_empty_queue() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(108, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for x in 101..=108 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 220);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }
        world.add_creature_think_check(monster);

        world.schedule_creature_wakeup(monster, 0);
        world.process_creature_todo(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go() || k.base().next_wakeup.is_some()),
            "drain with empty queue must idle-enqueue chase Go"
        );
    }

    /// Phase A — segment drain clears `has_follow_path` so idle repaths on next wakeup.
    #[test]
    fn idle_repaths_after_segment_drain_clears_follow_path() {
        let mut world = beat_driven_test_world();
        world.mechanics.profile.follow_repath_without_path = true;

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(108, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for x in 101..=108 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 220);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = true;
            m.base.walk_queue.clear();
        }

        world.finish_creature_todo_execute(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| !k.base().walk_queue.is_empty() || k.base().todo.has_go()),
            "772 finish must idle-repath after segment drain (has_follow_path cleared)"
        );
    }

    /// 772 active monster without follow enqueues roam Go from idle (TFS `getRandomStep` arm).
    #[test]
    fn idle_stimulus_enqueues_roam_for_active_monster_without_follow() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    TEST_SYNTHETIC_GROUND_WP,
                );
            }
        }

        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        register_distant_floor_spectator(&mut world, pos);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        world.monster_idle_stimulus(monster);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(todo.has_go(), "772 idle must enqueue roam Go");
        assert!(todo.has_wait(), "772 roam must enqueue Wait(1000) after Go");
    }

    /// Blocked dance / stand-still at melee goal must not force a chase repath on next idle.
    #[test]
    fn force_update_at_follow_goal_skips_idle_repath() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = true;
            m.base.force_update_follow_path = true;
            m.base.walk_queue.clear();
        }

        let (needs, reason) = world.monster_idle_chase_needs_repath(monster);
        assert!(!needs, "at-goal force_update must not schedule repath");
        assert!(reason.is_none());
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().force_update_follow_path),
            "stale force_update must be cleared at follow goal"
        );
    }

    /// 1098 regression — think still arms walk when not beat-driven.
    #[test]
    fn think_arm_still_runs_on_1098() {
        let mut world = minimal_world();
        assert!(!world.beat_driven_loop);

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for x in 101..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.attack_target = Some(player);
        }
        world.add_creature_think_check(monster);
        assert!(world.monster_set_follow_creature(monster, Some(player)));

        world.monster_native_on_think(monster, EVENT_CREATURE_THINK_INTERVAL_MS);

        let armed = world
            .creatures
            .get(monster)
            .is_some_and(|k| k.base().next_walk_check.is_some() || !k.base().walk_queue.is_empty());
        assert!(armed, "1098 think must still arm monster walk");
    }

    #[test]
    fn test_772_classify_roam_without_follow() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::Roam
        );
    }

    #[test]
    fn test_772_classify_flee_before_melee() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20;
            m.flee_opening_melee_dance_done = true;
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::Flee
        );
    }

    /// X3 — first adjacent idle while `runonhealth` flee is active still classifies `MeleeDance`.
    #[test]
    fn test_772_adjacent_fleeing_first_idle_melee_dances_then_flee() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster = insert_monster(&mut world, "Dragon", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20;
            m.flee_opening_melee_dance_done = false;
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.flee_opening_melee_dance_done = true;
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::Flee
        );
    }

    /// X3 — melee-only band (`targetdistance=1`) uses close `melee_dance`, not dist arms.
    #[test]
    fn test_772_classify_melee_dance_when_throw_not_possible_at_adjacent() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let mut cfg = MonsterAiConfig {
            is_hostile: true,
            target_distance: 1,
            melee_skill: 68,
            ..MonsterAiConfig::default()
        };
        cfg.spells.push(MonsterSpell {
            delay: 2000,
            range: 7,
            radius: 0,
            min_cycle: 0,
            shape: SpellShape::Victim,
            impact: SpellImpact::Damage {
                element: CombatType::Physical,
                base: 10,
                variation: 10,
            },
            shoot_effect: None,
            area_effect: None,
        });
        let monster = insert_monster_with_config(&mut world, "Dragon", mpos, 200, cfg);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        assert!(
            world.monster_can_use_attack(monster, mpos, player),
            "melee strike still counts for canUseAttack at cheb=1"
        );
        assert!(
            world.monster_throw_possible(monster, mpos, player),
            "ranged spell still in band at cheb=1"
        );
        assert!(
            !world.monster_idle_uses_dist_branch(monster, mpos, player, 1),
            "targetdistance=1 keeps close branch even when throw is possible"
        );
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance
        );
    }

    #[test]
    fn test_772_classify_master_follow() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MasterFollow
        );
    }

    #[test]
    fn test_772_classify_melee_vs_dist() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos_melee = Position::new(103, 100, 7);
        let ppos_dist = Position::new(106, 100, 7);
        for x in 99..=106 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let melee_monster = insert_monster_with_config(
            &mut world,
            "FixtureIdleChase772",
            mpos,
            200,
            MonsterAiConfig {
                is_hostile: false,
                ..MonsterAiConfig::default()
            },
        );
        let melee_player = insert_player(&mut world, test_player("Hero1", ppos_melee));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(melee_monster) {
            m.is_idle = false;
            m.base.follow_target = Some(melee_player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(melee_monster),
            MonsterIdleWalkBranch::MeleeChase
        );

        let dist_monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );
        let dist_player = insert_player(&mut world, test_player("Hero2", ppos_dist));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(dist_monster) {
            m.is_idle = false;
            m.base.follow_target = Some(dist_player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(dist_monster),
            MonsterIdleWalkBranch::DistChase
        );
    }

    #[test]
    fn test_772_classify_dist_dance_at_band() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::DistDance
        );
    }

    #[test]
    fn test_772_classify_melee_dance_adjacent() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance,
            "follow without attack_target may still rand(0,4) dance"
        );
    }

    #[test]
    fn test_772_attacking_posture_keeps_melee_dance_at_adjacent() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance,
            "ATTACKING melee still rand(0,4) dances at cheb==1"
        );
    }

    /// Flee arm uses `SearchFlightField` (single step), not a multi-step `TShortway` batch.
    #[test]
    fn test_772_flee_uses_flight_field_not_shortway() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(99, 100, 7), TEST_SYNTHETIC_GROUND_WP);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20;
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        let queue_len = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .len();
        assert!(
            queue_len <= 1,
            "flee idle must queue at most one flight-field step, got {queue_len}"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go() || k.base().next_wakeup.is_some()),
            "flee idle must enqueue Go"
        );
    }

    /// P0-4 — melee chase at cheb==2 uses reference `must:false, max:3`; trim stops at cheb≤1.
    ///
    /// Uses default spawn (`melee_skill==0`, state not `Attacking`) so classify stays `MeleeChase`;
    /// fist monsters in `Attacking` skip idle chase — see `test_e3_attacking_skips_idle_melee_chase`.
    #[test]
    fn test_772_melee_chase_cheb2_must_false_max_three() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};
        use crate::pathfinding::CHASE_PATH_MAX_STEPS;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeChase
        );
        let (max_steps, must_reach) = monster_idle_chase_step_budget(true, false, 2, 1);
        assert_eq!((max_steps, must_reach), (CHASE_PATH_MAX_STEPS, false));

        let outcome =
            world.monster_idle_chase_repath(monster, Some("idle_drain"), max_steps, must_reach);
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            1,
            "melee chase at cheb==2 must queue one step (trim at cheb≤1), not must:true NOWAY"
        );
    }

    /// A2 regression — farther melee chase still allows up to 3 steps.
    #[test]
    fn test_772_melee_chase_cheb4_three_steps() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let (max_steps, must_reach) = monster_idle_chase_step_budget(true, false, 4, 1);
        assert_eq!((max_steps, must_reach), (3, false));

        let outcome =
            world.monster_idle_chase_repath(monster, Some("idle_drain"), max_steps, must_reach);
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            3,
            "open-line melee chase at cheb==4 should queue three steps"
        );
    }

    /// A3 — dist chase step budget is `cheb - target_distance`, not global `CHASE_PATH_MAX_STEPS`.
    #[test]
    fn test_772_dist_chase_step_budget_from_target_distance() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos_band4 = Position::new(106, 100, 7);
        let ppos_band3 = Position::new(106, 110, 7);
        for x in 100..=106u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let dist_monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );
        let dist_player = insert_player(&mut world, test_player("Hero4", ppos_band4));
        world.map.register_creature_at(ppos_band4, dist_player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(dist_monster) {
            m.is_idle = false;
            m.base.follow_target = Some(dist_player);
            m.base.attack_target = Some(dist_player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(dist_monster),
            MonsterIdleWalkBranch::DistChase
        );
        let (max_steps, must_reach) = monster_idle_chase_step_budget(false, true, 6, 4);
        assert_eq!((max_steps, must_reach), (2, false));

        let outcome = world.monster_idle_chase_repath(
            dist_monster,
            Some("idle_drain"),
            max_steps,
            must_reach,
        );
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(dist_monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            2,
            "dist chase at cheb==6 with band 4 should queue two steps"
        );

        let mpos_band3 = Position::new(100, 110, 7);
        for x in 100..=106u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 110, 7), 150);
        }
        let band3_monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos_band3,
            200,
            dist_idle_monster_config(3),
        );
        let band3_player = insert_player(&mut world, test_player("Hero3", ppos_band3));
        world.map.register_creature_at(ppos_band3, band3_player);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(band3_monster) {
            m.is_idle = false;
            m.base.follow_target = Some(band3_player);
            m.base.attack_target = Some(band3_player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let (max_steps, must_reach) = monster_idle_chase_step_budget(false, true, 6, 3);
        assert_eq!((max_steps, must_reach), (3, false));
        let outcome = world.monster_idle_chase_repath(
            band3_monster,
            Some("idle_drain"),
            max_steps,
            must_reach,
        );
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(band3_monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            3,
            "dist chase at cheb==6 with band 3 should queue three steps"
        );
    }

    /// A2 / X5 — failed melee dance at band must not re-enqueue Go on 772 idle Hold.
    #[test]
    fn test_772_idle_hold_no_dance_poll() {
        use crate::tile::{flags as tilestate, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        ensure_walkable_tile(&mut world.map, ppos, 150);
        for (x, y) in [(99, 100), (101, 100), (100, 99), (100, 101)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(150),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance,
            "ATTACKING melee still attempts rand(0,4) dance at cheb==1"
        );

        world.monster_idle_stimulus(monster);

        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go()),
            "blocked dance tiles must not enqueue spurious Go"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_attack()),
            "stick-fight must enqueue Attack when dance cannot move"
        );
        assert!(world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .is_empty());
    }

    /// A0 — TShortway NOWAY clears chase target and enqueues roam Go same idle tick.
    #[test]
    fn test_772_chase_noway_clears_target_and_roams() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    TEST_SYNTHETIC_GROUND_WP,
                );
            }
        }
        ensure_walkable_tile(&mut world.map, ppos, 150);

        let monster = insert_monster_with_config(
            &mut world,
            "FixtureIdleChase772",
            mpos,
            200,
            MonsterAiConfig {
                is_hostile: false,
                ..MonsterAiConfig::default()
            },
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeChase,
            "non-fist fixture must use idle melee chase"
        );

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().follow_target.is_none()),
            "NOWAY must clear follow target"
        );
        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(
            todo.has_go(),
            "NOWAY must enqueue roam Go on same idle tick"
        );
        assert!(
            todo.has_wait(),
            "NOWAY roam must enqueue trailing Wait(1000)"
        );
    }

    /// A4 / X4 — 772 `getNextStep` must not inline flee when queue is empty.
    #[test]
    fn test_772_get_next_step_no_inline_flee_on_772() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.health = 10;
            m.run_away_health = 20;
            m.base.has_follow_path = true;
            m.base.walk_queue.clear();
        }

        let now = std::time::Instant::now();
        assert_eq!(
            world.monster_next_walk_step(monster, now),
            None,
            "772 getNextStep must defer flee to idle drain"
        );
    }

    /// A4 — dist_dance at keep band via idle only, not `getNextStep`.
    #[test]
    fn test_772_dist_dance_via_idle() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(99, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(101, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(100, 99, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), TEST_SYNTHETIC_GROUND_WP);

        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );
        let player = insert_player(&mut world, test_player("Hero", ppos));

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        for _ in 0..50 {
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
                m.base.walk_queue.clear();
                m.base.has_follow_path = false;
            }
            world.monster_idle_stimulus(monster);
            if let Some(dir) = world
                .creatures
                .get(monster)
                .and_then(|k| k.base().walk_queue.back().copied())
            {
                assert!(
                    matches!(dir, Direction::North | Direction::South),
                    "only North or South maintain target distance 4 from East-aligned target, got {:?}",
                    dir
                );
            }
        }
    }

    /// A5 / B2 — master follow Manhattan 2 enqueues Wait only (no Go).
    #[test]
    fn test_772_master_follow_manhattan_2_hold() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .is_empty(),
            "Manhattan 2 must hold without chase path"
        );
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go()),
            "Manhattan 2 must not enqueue Go"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait()),
            "Manhattan 2 must enqueue Wait(1000)"
        );
    }

    /// A5 / B2 — master follow Manhattan 3 enqueues Wait only.
    #[test]
    fn test_772_master_follow_manhattan_3_hold() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .is_empty(),
            "Manhattan 3 must hold without chase path"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait()),
            "Manhattan 3 must enqueue Wait(1000)"
        );
    }

    /// A5 — master follow beyond wait band queues up to 3 steps.
    #[test]
    fn test_772_master_follow_manhattan_5_chases() {
        use crate::monster_ai::MonsterIdleChaseRepathOutcome;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for x in 100..=105u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let master = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        world.map.register_creature_at(ppos, master);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.master = Some(master);
            m.base.follow_target = Some(master);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let outcome = world.monster_idle_master_follow(monster, Some("idle_drain"));
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len()
                <= 3,
            "master follow must cap at 3 steps"
        );
    }

    #[test]
    fn test_772_wait_schedules_1000ms_wakeup() {
        let mut world = beat_driven_test_world();
        world.server_ms = 200;
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);

        world.idle_enqueue_wait_and_start(monster, MONSTER_IDLE_WAIT_MS);
        world.run_monster_todo_execute(monster);

        assert!(world.creatures.get(monster).unwrap().base().todo.is_empty());
        assert_eq!(
            world.creatures.get(monster).unwrap().base().next_wakeup,
            Some(200 + MONSTER_IDLE_WAIT_MS)
        );
    }

    /// Regression: multi-step chase must drain the full `walk_queue`, not freeze after one Go.
    #[test]
    fn test_772_multi_step_chase_continues_after_first_go() {
        use crate::monster_ai::{monster_idle_chase_step_budget, MonsterIdleChaseRepathOutcome};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        let (max_steps, must_reach) = monster_idle_chase_step_budget(true, false, 4, 1);
        assert_eq!((max_steps, must_reach), (3, false));
        let outcome =
            world.monster_idle_chase_repath(monster, Some("idle_drain"), max_steps, must_reach);
        assert_eq!(outcome, MonsterIdleChaseRepathOutcome::PathQueued);
        assert_eq!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len(),
            3
        );

        world.enqueue_creature_go(monster);
        world.schedule_immediate_todo_wakeup(monster);
        world.process_creature_todo(monster);

        let pos_after_one = world.creatures.get(monster).unwrap().position();
        assert!(
            pos_after_one.x > mpos.x,
            "first Go must move monster east from {:?}, got {:?}",
            mpos,
            pos_after_one
        );

        let wq_after_one = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .walk_queue
            .len();
        assert!(
            wq_after_one >= 1,
            "after first step walk_queue should still have pending steps, got {wq_after_one}"
        );

        // Drain all scheduled wakeups until monster reaches player column or stalls.
        for _ in 0..20 {
            let wakeup = world
                .creatures
                .get(monster)
                .and_then(|k| k.base().next_wakeup);
            let Some(wu) = wakeup else {
                break;
            };
            world.server_ms = wu;
            while world
                .todo_queue
                .peek()
                .is_some_and(|e| e.execution_time <= world.server_ms)
            {
                world.drain_todo_queue();
            }
        }

        let final_pos = world.creatures.get(monster).unwrap().position();
        assert!(
            final_pos.x > pos_after_one.x,
            "multi-step chase must continue past first tile (after one={:?}, final={:?}, wq={})",
            pos_after_one,
            final_pos,
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .walk_queue
                .len()
        );
    }

    #[test]
    fn test_772_roam_pacing_via_wait_not_last_step() {
        let mut world = beat_driven_test_world();
        world.server_ms = 0;
        let pos = Position::new(100, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    TEST_SYNTHETIC_GROUND_WP,
                );
            }
        }
        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        register_distant_floor_spectator(&mut world, pos);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        world.monster_idle_stimulus(monster);
        assert!(world.creatures.get(monster).unwrap().base().todo.has_go());

        world.run_monster_todo_execute(monster);
        assert!(
            world.creatures.get(monster).unwrap().base().todo.is_empty(),
            "Go then Wait chain must drain Go and schedule Wait"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .next_wakeup
                .unwrap()
                >= MONSTER_IDLE_WAIT_MS
        );

        world.monster_idle_stimulus(monster);
        assert!(
            !world.creatures.get(monster).unwrap().base().todo.has_go(),
            "Wait in flight must block immediate re-roam"
        );
    }

    #[test]
    fn test_772_dist_flee_fail_enqueues_wait() {
        use tfs_rust_common::enums::ZoneType;

        use crate::tile::{flags as tilestate, Tile, TileBody};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for (x, y) in [(99, 100), (100, 99), (100, 101)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(TEST_SYNTHETIC_GROUND_WP),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::DistFlee
        );

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait() && !k.base().todo.has_go()),
            "dist_flee fail must enqueue Wait only"
        );
    }

    #[test]
    fn test_772_dist_dance_enqueues_go_and_wait() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        for x in 100..=104 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        ensure_walkable_tile(&mut world.map, Position::new(99, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(100, 99, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        let monster = insert_monster_with_config(
            &mut world,
            "Rat",
            mpos,
            200,
            dist_idle_monster_config(4),
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::DistDance
        );

        let mut got_go = false;
        for _ in 0..50 {
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
                m.base.walk_queue.clear();
                m.base.todo.queue.clear();
                m.base.has_follow_path = false;
                m.base.next_wakeup = None;
            }
            world.monster_idle_stimulus(monster);
            if world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_go())
            {
                got_go = true;
                break;
            }
        }

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(got_go, "dist_dance must enqueue Go");
        assert!(todo.has_wait(), "dist_dance must enqueue Wait after Go");
    }

    #[test]
    fn test_772_get_next_step_no_roam_on_beat_loop() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                ensure_walkable_tile(
                    &mut world.map,
                    Position::new((100 + dx) as u16, (100 + dy) as u16, 7),
                    TEST_SYNTHETIC_GROUND_WP,
                );
            }
        }
        let monster = insert_monster(&mut world, "Wolf", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
        }

        let now = std::time::Instant::now();
        assert_eq!(
            world.monster_next_walk_step(monster, now),
            None,
            "772 getNextStep must not pick roam step inline"
        );
    }

    #[test]
    fn test_772_attack_from_idle_queue() {
        use tfs_rust_common::enums::ZoneType;

        use crate::tile::{flags as tilestate, Tile, TileBody};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for (x, y) in [(99, 100), (100, 99), (100, 101)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(TEST_SYNTHETIC_GROUND_WP),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_attack()),
            "hostile melee at cheb==1 must enqueue Attack without spell-range canUseAttack"
        );
    }

    /// P0-2 — change-target ticks advance on `ProcessCreatures` only, not each idle drain.
    #[test]
    fn test_772_change_target_only_on_process_creatures() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(105, 100, 7);
        for x in 100..=105u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }

        let config = MonsterAiConfig {
            change_target_speed: 4_000,
            change_target_chance: 100,
            ..Default::default()
        };
        let monster = insert_monster_with_config(&mut world, "Rat", mpos, 200, config);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.target_change_ticks = 0;
            m.target_change_cooldown = 0;
        }

        for _ in 0..5 {
            world.monster_idle_stimulus(monster);
        }
        let ticks_after_idle = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m.target_change_ticks,
            _ => 0,
        };
        assert_eq!(
            ticks_after_idle, 0,
            "idle drain must not advance change-target ticks on 772"
        );

        world.add_creature_think_check(monster);
        world.process_creatures_772();
        let ticks_after_think = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m.target_change_ticks,
            _ => 0,
        };
        assert_eq!(
            ticks_after_think, 0,
            "772 ProcessCreatures must not run TFS change-target rolls (no `onThinkTarget` in `crnonpl.cc`)"
        );
    }

    /// P0-3 — melee stick-fight enqueues Attack without trailing 1 s Wait.
    #[test]
    fn test_772_melee_stick_fight_no_wait_after_attack() {
        use tfs_rust_common::enums::ZoneType;

        use crate::creature_todo::{CreatureAction, MONSTER_IDLE_WAIT_MS};
        use crate::tile::{flags as tilestate, Tile, TileBody};

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        for (x, y) in [(99, 100), (100, 99), (100, 101)] {
            world.map.insert_tile(
                Position::new(x, y, 7),
                Tile::Normal(TileBody {
                    ground: Some(TEST_SYNTHETIC_GROUND_WP),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }

        world.monster_idle_stimulus(monster);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(todo.has_attack(), "melee stick-fight must enqueue Attack");
        assert!(
            !todo.queue.iter().any(|a| {
                matches!(a, CreatureAction::Wait { delay_ms } if *delay_ms == MONSTER_IDLE_WAIT_MS)
            }),
            "melee stick-fight must not enqueue trailing 1 s Wait after Attack"
        );
    }

    #[test]
    fn test_772_think_skips_creature_on_attacking() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.opponent_ids.push(player);
            m.base.attack_target = Some(player);
            m.base.follow_target = Some(player);
        }
        world.add_creature_think_check(monster);

        world.process_creatures_772();

        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_attack()),
            "772 ~1 Hz think must not enqueue Attack — idle todo path owns combat tail"
        );
    }

    fn e1_melee_target_setup(world: &mut GameWorld, melee_skill: i32) -> (CreatureId, CreatureId) {
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        let mut player = test_player("Hero", ppos);
        player.base.health = 500;
        player.base.max_health = 500;
        let player = insert_player(world, player);
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = melee_skill;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
        }
        (monster, player)
    }

    #[test]
    fn test_e1_melee_monster_enters_attacking_on_idle() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 15);

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world.creatures.get(monster).and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.state),
                _ => None,
            }),
            Some(MonsterState::Attacking),
            "hostile melee with target must enter Attacking on idle drain"
        );
    }

    #[test]
    fn test_e1_idle_reset_reasserts_attacking_each_tick() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 15);

        for tick in 0..2 {
            if tick > 0 {
                if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
                    m.base.todo.queue.clear();
                    m.base.walk_queue.clear();
                    m.base.next_wakeup = None;
                }
            }
            world.monster_idle_stimulus(monster);
            assert_eq!(
                world.creatures.get(monster).and_then(|k| match k {
                    CreatureKind::Monster(m) => Some(m.state),
                    _ => None,
                }),
                Some(MonsterState::Attacking),
                "reset→Idle then walk must re-set Attacking when walk section runs"
            );
        }
    }

    #[test]
    fn test_e1_under_attack_promoted_to_attacking_in_walk_section() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 15);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::UnderAttack;
        }

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world.creatures.get(monster).and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.state),
                _ => None,
            }),
            Some(MonsterState::Attacking),
            "top reset preserves UnderAttack; walk prelude promotes to Attacking — crnonpl.cc:2705"
        );
    }

    #[test]
    fn test_e1_no_attacking_without_melee_skill() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e1_melee_target_setup(&mut world, 0);

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world.creatures.get(monster).and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.state),
                _ => None,
            }),
            Some(MonsterState::Idle),
            "melee_skill==0 must not enter Attacking"
        );
    }

    #[test]
    fn test_e1_panic_blocks_attacking_set() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
            m.state = MonsterState::Panic;
        }

        world.monster_idle_stimulus(monster);

        assert_eq!(
            world.creatures.get(monster).and_then(|k| match k {
                CreatureKind::Monster(m) => Some(m.state),
                _ => None,
            }),
            Some(MonsterState::Panic),
            "PANIC must block Attacking transition"
        );
    }

    fn e5_apply_player_hit(
        world: &mut GameWorld,
        monster: CreatureId,
        player: CreatureId,
        damage: i32,
    ) {
        let applied = world.combat_execute_with_stimulus(
            Some(player),
            monster,
            &CombatDamage {
                primary: (CombatType::Physical, -damage),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
        );
        assert!(applied, "combat_execute_with_stimulus must apply HP loss");
    }

    #[test]
    fn test_e5_idle_with_target_hit_becomes_under_attack() {
        let mut world = beat_driven_test_world();
        let (monster, player) = e1_melee_target_setup(&mut world, 15);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Idle;
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        e5_apply_player_hit(&mut world, monster, player, 5);

        let m = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.state,
            MonsterState::UnderAttack,
            "idle rat with target must flip to UnderAttack on hit"
        );
        assert!(
            m.base
                .todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms: 0 })),
            "DamageStimulus must ToDoYield (Wait(0)) — cract.cc:1001"
        );
        assert!(
            m.base.next_wakeup.is_some(),
            "yield must schedule immediate todo wakeup"
        );
    }

    #[test]
    fn test_e5_sleeping_no_target_hit_becomes_panic_and_yields() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Sleeping;
            m.is_idle = true;
            m.base.clear_targets();
            m.opponent_ids.clear();
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        e5_apply_player_hit(&mut world, monster, player, 3);

        let m = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.state,
            MonsterState::Panic,
            "sleeping rat without target → PANIC"
        );
        assert!(!m.is_idle, "PANIC must wake monster from idle posture");
        assert!(
            m.opponent_ids.contains(&player),
            "attacker must be recorded in opponent_ids"
        );
        assert!(
            m.base
                .todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms: 0 })),
            "sleeping hit must ToDoYield"
        );
    }

    #[test]
    fn test_e5_panic_dances_without_low_health() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Sleeping;
            m.is_idle = true;
            m.run_away_health = 0;
            m.base.health = 200;
            m.base.clear_targets();
            m.opponent_ids.clear();
        }

        e5_apply_player_hit(&mut world, monster, player, 1);

        assert!(
            world.creatures.get(monster).is_some_and(|k| match k {
                CreatureKind::Monster(m) => {
                    m.state == MonsterState::Panic && !m.is_fleeing()
                }
                _ => false,
            }),
            "PANIC must not gate IsFleeing — crnonpl.cc:3136"
        );
        // C++ `DamageStimulus` does not set `Target`; idle `Strategy[]` picks on next drain.
        world.monster_idle_stimulus(monster);
        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::MeleeDance
        );
    }

    /// C++ `%5` case 2/3 map to North/South dest tiles — `crnonpl.cc:2817-2818`.
    #[test]
    fn test_772_dance_dir_order_matches_cpp() {
        use crate::sim_glibc_rand::DANCE_DIR_ORDER;
        use tfs_rust_common::Position;

        let pos = Position::new(32361, 32290, 7);
        assert_eq!(
            pos.offset(DANCE_DIR_ORDER[2].unwrap()),
            Position::new(32361, 32289, 7),
            "case 2 must step north (DestY-=1)"
        );
        assert_eq!(
            pos.offset(DANCE_DIR_ORDER[3].unwrap()),
            Position::new(32361, 32291, 7),
            "case 3 must step south (DestY+=1)"
        );
    }

    #[test]
    fn test_e5_rehit_attacking_no_redundant_yield() {
        let mut world = beat_driven_test_world();
        let (monster, player) = e1_melee_target_setup(&mut world, 15);

        world.monster_idle_stimulus(monster);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            assert_eq!(m.state, MonsterState::Attacking);
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        e5_apply_player_hit(&mut world, monster, player, 2);
        e5_apply_player_hit(&mut world, monster, player, 2);

        let m = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.state,
            MonsterState::Attacking,
            "re-hit while Attacking must keep Attacking"
        );
        assert!(
            m.base.todo.queue.is_empty(),
            "re-hit with unchanged state must not storm ToDoYield"
        );
        assert!(
            m.base.next_wakeup.is_none(),
            "no redundant yield wakeup when state unchanged"
        );
    }

    fn e3_melee_target_at_cheb2(
        world: &mut GameWorld,
        melee_skill: i32,
    ) -> (CreatureId, CreatureId) {
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), 150);
        }
        let player = insert_player(world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = melee_skill;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
            m.base.todo.queue.clear();
        }
        (monster, player)
    }

    #[test]
    fn test_e3_attacking_skips_idle_melee_chase() {
        let mut world = beat_driven_test_world();
        let (monster, _player) = e3_melee_target_at_cheb2(&mut world, 15);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
        }

        assert_eq!(
            world.monster_idle_classify_walk_branch(monster),
            MonsterIdleWalkBranch::Hold,
            "ATTACKING at cheb==2 must not use idle MeleeChase"
        );
    }

    #[test]
    fn test_e3_attack_path_enqueues_close_chase_at_cheb2() {
        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let (monster, _player) = e3_melee_target_at_cheb2(&mut world, 15);

        world.monster_idle_stimulus(monster);

        let m = match world.creatures.get(monster) {
            Some(CreatureKind::Monster(m)) => m,
            _ => panic!("expected monster"),
        };
        assert_eq!(
            m.chase_mode,
            MonsterChaseMode::Close,
            "melee ATTACKING must set CHASE_MODE_CLOSE"
        );
        assert!(
            !m.base.walk_queue.is_empty(),
            "attack-path CanToDoAttack must populate walk_queue at cheb==2"
        );
        let todo = &m.base.todo;
        assert!(todo.has_go(), "attack tail must enqueue Go before Attack");
        assert!(todo.has_attack(), "attack tail must enqueue Attack");
        assert!(
            !todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms: 100 })),
            "fist ToDoAttack skips Wait(100) when GetDistance()==1 (cract.cc:1327)"
        );
        let go_idx = todo
            .queue
            .iter()
            .position(|a| matches!(a, CreatureAction::Go))
            .expect("Go in queue");
        let attack_idx = todo
            .queue
            .iter()
            .position(|a| matches!(a, CreatureAction::Attack))
            .expect("Attack in queue");
        assert!(
            go_idx < attack_idx,
            "ToDoAttack order: Go before Attack (cract.cc:1325-1334)"
        );
    }

    fn e2_adjacent_combat_setup(
        world: &mut GameWorld,
        melee_skill: i32,
        melee_attack: i32,
    ) -> (CreatureId, CreatureId) {
        let (monster, player) = e1_melee_target_setup(world, melee_skill);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.melee_attack = melee_attack;
        }
        (monster, player)
    }

    fn e2_run_attack_todo(world: &mut GameWorld, monster: CreatureId) {
        world.enqueue_creature_attack(monster);
        world.schedule_immediate_todo_wakeup(monster);
        world.run_monster_todo_execute(monster);
    }

    fn e2_drain_until_idle(world: &mut GameWorld, monster: CreatureId) {
        for _ in 0..30 {
            let wakeup = world
                .creatures
                .get(monster)
                .and_then(|k| k.base().next_wakeup);
            let Some(wu) = wakeup else {
                break;
            };
            world.server_ms = wu;
            while world
                .todo_queue
                .peek()
                .is_some_and(|e| e.execution_time <= world.server_ms)
            {
                world.drain_todo_queue();
            }
        }
    }

    #[test]
    fn test_e2_melee_damage_and_damage_map() {
        use crate::max_melee_damage_monster;

        let mut world = beat_driven_test_world();
        world.server_ms = 1000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);
        let hp_before = world.creatures.get(player).unwrap().base().health;

        e2_run_attack_todo(&mut world, monster);

        let hp_after = world.creatures.get(player).unwrap().base().health;
        assert!(hp_after < hp_before, "adjacent melee must reduce target HP");
        let dealt = (hp_before - hp_after) as u64;
        assert!(
            dealt <= max_melee_damage_monster(15, 7) as u64,
            "damage must not exceed max roll"
        );
        assert_eq!(
            world
                .creatures
                .get(player)
                .unwrap()
                .base()
                .damage_map
                .get(&monster)
                .copied(),
            Some(dealt),
            "damage_map must attribute dealt HP to attacker"
        );
    }

    #[test]
    fn test_e2_attack_cadence_2000ms() {
        let mut world = beat_driven_test_world();
        world.server_ms = 5000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);

        e2_run_attack_todo(&mut world, monster);
        let hp_after_first = world.creatures.get(player).unwrap().base().health;
        let earliest = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .earliest_attack_ms;
        assert_eq!(earliest, 5000 + 2000, "CloseAttack must DelayAttack(2000)");

        world.server_ms = earliest - 1;
        e2_run_attack_todo(&mut world, monster);
        assert_eq!(
            world.creatures.get(player).unwrap().base().health,
            hp_after_first,
            "attack must not land before cadence elapses"
        );

        world.server_ms = earliest;
        e2_drain_until_idle(&mut world, monster);
        let hp_second = world
            .creatures
            .get(player)
            .map(|k| k.base().health)
            .expect("player must remain in world");
        assert!(
            hp_second < hp_after_first,
            "second hit must land after 2000 ms cadence"
        );
    }

    #[test]
    fn test_e2_melee_adjacent_enqueues_attack_without_wait() {
        use crate::creature::monster_weapon_attack_distance;

        let mut world = beat_driven_test_world();
        let (monster, _player) = e2_adjacent_combat_setup(&mut world, 15, 7);
        let (melee_skill, has_ranged) = world
            .creatures
            .get(monster)
            .map(|k| match k {
                CreatureKind::Monster(m) => (m.melee_skill, m.spells.iter().any(|s| s.range > 1)),
                _ => (0, false),
            })
            .unwrap();
        assert_eq!(monster_weapon_attack_distance(melee_skill, has_ranged), 1);

        assert!(world.enqueue_creature_attack(monster));

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 1);
        assert!(matches!(todo.queue[0], CreatureAction::Attack));
    }

    #[test]
    fn test_e2_wait_100_before_attack_when_weapon_range_not_close() {
        use crate::creature::monster_weapon_attack_distance;

        assert_eq!(monster_weapon_attack_distance(0, true), 3);
        assert_eq!(monster_weapon_attack_distance(15, true), 1);

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        let spell = MonsterSpell {
            delay: 4,
            range: 5,
            radius: 0,
            min_cycle: 6,
            shape: SpellShape::Victim,
            impact: SpellImpact::Condition {
                condition: ConditionType::Poison,
                cycle: 20,
                min_cycle: 6,
            },
            shoot_effect: None,
            area_effect: None,
        };
        let mut cfg = MonsterAiConfig::default();
        cfg.melee_skill = 0;
        cfg.spells = vec![spell];
        let monster = insert_monster_with_config(&mut world, "Cobra", mpos, 200, cfg);

        if monster_weapon_attack_distance(0, true) != 1 {
            world.enqueue_creature_wait(monster, 100);
        }
        world.enqueue_creature_attack(monster);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert_eq!(todo.queue.len(), 2);
        assert!(matches!(
            todo.queue[0],
            CreatureAction::Wait { delay_ms: 100 }
        ));
        assert!(matches!(todo.queue[1], CreatureAction::Attack));
    }

    fn e4_cobra_config() -> MonsterAiConfig {
        use std::path::Path;
        use tfs_rust_content::items::ItemDatabase;
        use tfs_rust_content::monsters::MonsterDatabase;

        let manifest = env!("CARGO_MANIFEST_DIR");
        let items = ItemDatabase {
            items: Default::default(),
            client_to_server: Default::default(),
        };
        let db = MonsterDatabase::load_dir(&Path::new(manifest).join("../../data/monster"), &items)
            .expect("load monsters");
        let mtype = db.monsters.get("cobra").cloned().expect("cobra type");
        MonsterAiConfig::from_monster_type(&mtype)
    }

    #[test]
    fn test_e4_cobra_poison_at_range() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        let mut cfg = e4_cobra_config();
        cfg.spells[0].delay = 1;
        let monster = insert_monster_with_config(&mut world, "Cobra", mpos, 200, cfg);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.state = MonsterState::Idle;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        let mut poisoned = false;
        for attempt in 0..64 {
            if attempt > 0 {
                // Delay gate: rand() % 4 == 0 — retry until cast fires.
            }
            world.monster_idle_stimulus(monster);
            poisoned = world.creatures.get(player).is_some_and(|k| {
                k.base()
                    .active_conditions
                    .iter()
                    .any(|c| c.ctype == ConditionType::Poison)
            });
            if poisoned {
                break;
            }
        }
        assert!(
            poisoned,
            "cobra must apply poison condition to player at Chebyshev distance 3 within spell range 5"
        );
    }

    #[test]
    fn test_e4_casting_runs_after_target_acquire() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        let mut cfg = e4_cobra_config();
        cfg.spells[0].delay = 1;
        let monster = insert_monster_with_config(&mut world, "Cobra", mpos, 200, cfg);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.state = MonsterState::Idle;
            m.strategy_nearest = 100;
            m.strategy_health = 0;
            m.strategy_damage = 0;
        }

        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .follow_target
                .is_none(),
            "precondition: no target before idle"
        );

        world.monster_idle_stimulus(monster);

        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .follow_target
                .is_some(),
            "acquire must pick target same idle cycle"
        );
        assert!(
            world.creatures.get(player).is_some_and(|k| {
                k.base()
                    .active_conditions
                    .iter()
                    .any(|c| c.ctype == ConditionType::Poison)
            }),
            "cast must run after acquire on the same idle cycle when delay=1"
        );
    }

    #[test]
    fn test_e4_spell_delay_gate() {
        let mut world = beat_driven_test_world();
        world.seed_parity_rng(772);
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);

        let mut cfg = e4_cobra_config();
        cfg.spells[0].delay = 4;
        let monster = insert_monster_with_config(&mut world, "Cobra", mpos, 200, cfg);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.state = MonsterState::Idle;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
        }

        let mut casts = 0u32;
        for _ in 0..40 {
            world.server_ms = world.server_ms.saturating_add(200);
            if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
                p.base.active_conditions.clear();
            }
            world.monster_idle_stimulus(monster);
            let poisoned = world.creatures.get(player).is_some_and(|k| {
                k.base()
                    .active_conditions
                    .iter()
                    .any(|c| c.ctype == ConditionType::Poison)
            });
            if poisoned {
                casts += 1;
            }
        }
        assert!(
            casts >= 4 && casts <= 16,
            "delay=4 gate should yield roughly 1-in-4 cast attempts over 40 idles, got {casts}"
        );
    }

    #[test]
    fn test_e2_attack_deferred_until_cadence() {
        let mut world = beat_driven_test_world();
        world.server_ms = 2000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);
        let hp_before = world.creatures.get(player).unwrap().base().health;

        e2_run_attack_todo(&mut world, monster);
        let hp_after_first = world.creatures.get(player).unwrap().base().health;
        assert!(hp_after_first < hp_before, "first attack must deal damage");

        world.enqueue_creature_attack(monster);
        world.schedule_immediate_todo_wakeup(monster);
        world.run_monster_todo_execute(monster);
        assert_eq!(
            world.creatures.get(player).unwrap().base().health,
            hp_after_first,
            "immediate re-attack must defer without damage"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .next_wakeup
                .is_some(),
            "deferred attack must schedule a wakeup"
        );
    }

    /// Regression: adjacent melee must not freeze after first hit while target stands still.
    ///
    /// C++ always enqueues `ToDoAttack` at the idle tail; `TDAttack` arms the cadence wakeup.
    #[test]
    fn test_e2_melee_adjacent_does_not_freeze_after_first_strike() {
        let mut world = beat_driven_test_world();
        world.server_ms = 5000;
        let (monster, player) = e2_adjacent_combat_setup(&mut world, 15, 7);
        let hp_before = world.creatures.get(player).unwrap().base().health;

        e2_run_attack_todo(&mut world, monster);
        let hp_after_first = world.creatures.get(player).unwrap().base().health;
        assert!(hp_after_first < hp_before, "first attack must deal damage");

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.todo.has_attack() || base.next_wakeup.is_some(),
            "adjacent melee on cooldown must keep Attack or cadence wakeup armed (not freeze)"
        );

        let earliest = world
            .creatures
            .get(monster)
            .unwrap()
            .base()
            .earliest_attack_ms;
        e2_drain_until_idle(&mut world, monster);
        let hp_second = world
            .creatures
            .get(player)
            .map(|k| k.base().health)
            .expect("player must remain in world");
        assert!(
            hp_second < hp_after_first,
            "second hit must land after cadence without target moving"
        );
        assert_eq!(
            earliest,
            5000 + 2000,
            "cadence must remain DelayAttack(2000) after idle re-enqueue"
        );
    }

    /// Empty `walk_queue` + no `TDAttack` — follow-move must not idle-repath (`crmain.cc:919-961`;
    /// lesson 37: empty queue defers to idle segment drain).
    #[test]
    fn test_chase_empty_queue_attacking_does_not_idle_repath_on_target_kite() {
        let mut world = beat_driven_test_world();
        world.server_ms = 5000;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let ppos_kited = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.base.walk_queue.clear();
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
            m.base.has_follow_path = false;
            m.base.force_update_follow_path = false;
            m.base.earliest_attack_ms = world.server_ms + 2000;
        }

        world.monster_dispatch_creature_move(player, ppos, ppos_kited);

        let base = world.creatures.get(monster).unwrap().base();
        assert_eq!(
            base.next_wakeup, None,
            "empty queue must not schedule idle repath on kite"
        );
        assert!(base.walk_queue.is_empty());
        assert!(base.todo.is_empty());
        assert!(!base.force_update_follow_path);
        assert_eq!(
            world.creatures.get(monster).unwrap().position(),
            mpos,
            "no idle repath — position unchanged until idle drain"
        );
        assert_eq!(
            base.follow_target,
            Some(player),
            "kite must not drop follow"
        );
    }

    /// `TDAttack` armed close-chase — `CreatureMoveStimulus` re-queues Wait+Attack (`crmain.cc:946-961`).
    #[test]
    fn test_chase_combat_move_stimulus_rearms_attack_on_target_kite() {
        use crate::creature_todo::CreatureAction;

        let mut world = beat_driven_test_world();
        world.server_ms = 5000;
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let ppos_kited = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.base.walk_queue.clear();
            m.base.todo.queue.push_back(CreatureAction::Attack);
            m.base.todo.locked = false;
            m.base.next_wakeup = None;
            m.base.earliest_attack_ms = world.server_ms;
        }

        world.monster_dispatch_creature_move(player, ppos, ppos_kited);

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.todo.has_attack(),
            "combat move stimulus must re-arm TDAttack after target kites away"
        );
        assert!(
            !base.todo.is_empty(),
            "combat re-arm must enqueue Wait+Attack actions"
        );
        assert_eq!(
            base.follow_target,
            Some(player),
            "combat re-arm must keep follow"
        );
    }

    /// Dist at keep-band: target flee must inline-chase, not sit in goal `ToDoWait(1000)`.
    #[test]
    fn test_772_dist_target_flee_inline_chase_after_goal_wait() {
        use crate::creature_todo::CreatureAction;
        use crate::test_world::support::dist_idle_monster_config;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(104, 100, 7);
        let ppos_fled = Position::new(105, 100, 7);
        for x in 100..=105 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster_with_config(
            &mut world,
            "Hunter",
            mpos,
            200,
            dist_idle_monster_config(4),
        );
        world.map.register_creature_at(mpos, monster);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.base.walk_queue.clear();
            m.base.todo.queue.clear();
            m.base.has_follow_path = true;
        }

        world.monster_idle_stimulus(monster);
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait()),
            "at dist band idle arms trailing wait"
        );

        world.map.unregister_creature_at(ppos, player);
        world.map.register_creature_at(ppos_fled, player);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.base.position = ppos_fled;
        }
        world.monster_dispatch_creature_move(player, ppos, ppos_fled);

        let todo = &world.creatures.get(monster).unwrap().base().todo;
        assert!(
            todo.has_go(),
            "target leaving dist band must arm chase Go immediately"
        );
        assert!(
            !todo.queue.iter().any(|a| matches!(a, CreatureAction::Wait { delay_ms: 1000 })),
            "goal wait must be preempted when target flees"
        );
    }

    /// Close chase: pending `ToDoGo` must not block restep when target leaves cheb 1.
    #[test]
    fn test_772_close_chase_pending_go_clears_on_target_flee() {
        use crate::creature_todo::CreatureAction;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let ppos_fled = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        world.map.register_creature_at(mpos, monster);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.base.walk_queue.clear();
            m.base.todo.queue.push_back(CreatureAction::Go);
            m.base.todo.locked = false;
            m.base.has_follow_path = true;
            m.base.earliest_attack_ms = world.server_ms + 2000;
        }

        world.monster_dispatch_creature_move(player, ppos, ppos_fled);

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.todo.has_go(),
            "pending goal Go must be replaced with chase Go when target flees"
        );
        assert!(
            !base.todo.has_attack() || base.todo.queue.len() > 1,
            "stale single-action queue must be rebuilt for chase"
        );
    }

    /// Attack-path `TShortway` fail must NOWAY-clear target and not enqueue undeliverable Attack.
    #[test]
    fn test_chase_freeze_attack_path_noway_clears_target() {
        use crate::map::Map;
        use crate::test_world::support::{beat_driven_world, insert_monster_with_config};
        use crate::tile::{Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        fn sight_open_unwalkable(map: &mut Map, pos: Position) {
            map.insert_tile(
                pos,
                Tile::Normal(TileBody {
                    ground: None,
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: 0,
                    zone: ZoneType::Normal,
                }),
            );
        }

        let mut world = beat_driven_world();
        let mpos = Position::new(100, 100, 7);
        let mid = Position::new(101, 100, 7);
        let ppos = Position::new(102, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 150);
        sight_open_unwalkable(&mut world.map, mid);
        ensure_walkable_tile(&mut world.map, ppos, 150);

        let monster =
            insert_monster_with_config(&mut world, "Rat", mpos, 200, MonsterAiConfig::default());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
        }

        assert_eq!(
            world.monster_combat_enqueue_close_chase_go(monster),
            MonsterCombatCloseChaseEnqueue::Retry,
            "attack-path close chase must Retry when TShortway fails"
        );
        let base = world.creatures.get(monster).unwrap().base();
        assert_eq!(
            base.follow_target,
            Some(player),
            "Retry must keep chase target"
        );
        assert_eq!(base.attack_target, Some(player));
        assert!(
            !base.todo.has_attack(),
            "Retry must not leave undeliverable Attack"
        );

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.chase_mode = MonsterChaseMode::Close;
            m.base.todo.queue.clear();
        }
        assert!(
            matches!(
                world.monster_enqueue_todo_attack_actions(monster),
                MonsterEnqueueAttackResult::Retry | MonsterEnqueueAttackResult::Failed,
            ),
            "blocked chase must not enqueue Attack"
        );
        assert!(
            !world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .todo
                .has_attack(),
            "blocked chase must not leave Attack on the todo queue"
        );
    }

    /// Blocked mid-batch step must idle-repath instead of re-arming stale walk_queue dirs.
    #[test]
    fn test_chase_freeze_force_update_clears_stale_walk_batch() {
        use std::collections::VecDeque;
        use tfs_rust_common::enums::Direction;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.walk_queue = VecDeque::from([Direction::East, Direction::East]);
            m.base.force_update_follow_path = true;
            m.base.todo.queue.clear();
        }

        world.finish_creature_todo_execute(monster);

        assert!(
            world.creatures.get(monster).is_some_and(|k| {
                let base = k.base();
                base.walk_queue.is_empty() || base.todo.has_go()
            }),
            "force_update after blocked step must clear stale batch or idle-repath"
        );
    }

    #[test]
    fn test_e3_attack_enqueue_succeeds_when_close_go_already_queued() {
        use std::collections::VecDeque;
        use tfs_rust_common::enums::Direction;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.walk_queue = VecDeque::from([Direction::East]);
            m.base.todo.queue.push_back(CreatureAction::Go);
        }

        assert_eq!(
            world.monster_enqueue_todo_attack_actions(monster),
            MonsterEnqueueAttackResult::Enqueued,
            "mid-batch close Go must not fail attack enqueue"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .todo
                .has_attack(),
            "Attack must append when close Go already queued"
        );
    }

    #[test]
    fn test_772_attacking_idle_tail_label_when_close_chase_skipped() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        for x in 100..=101u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Cyclops", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 50;
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.todo.queue.clear();
            m.base.walk_queue.clear();
        }

        assert_eq!(
            world.monster_enqueue_todo_attack_actions(monster),
            MonsterEnqueueAttackResult::Enqueued,
            "ATTACKING at cheb==1 must enqueue attack without close-chase Go"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .unwrap()
                .base()
                .todo
                .has_attack(),
            "idle tail must append ToDoAttack when close chase is skipped"
        );
        assert!(
            world.monster_idle_skip_idle_melee_chase(monster),
            "ATTACKING posture must skip idle melee chase"
        );
    }

    #[test]
    fn test_chase_blocked_follower_rewakes_when_blocker_moves() {
        let mut world = beat_driven_test_world();
        let bpos = Position::new(100, 100, 7);
        let apos = Position::new(101, 100, 7);
        let ppos = Position::new(103, 100, 7);
        let apos_moved = Position::new(101, 101, 7);
        for pos in [bpos, apos, apos_moved, ppos] {
            ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        }
        ensure_walkable_tile(&mut world.map, Position::new(100, 101, 7), TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, Position::new(102, 100, 7), TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let blocker = insert_monster(&mut world, "Rat", apos, 200);
        let follower = insert_monster(&mut world, "Rat", bpos, 200);
        for id in [blocker, follower] {
            if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(id) {
                m.is_idle = false;
                m.is_hostile = true;
                m.melee_skill = 15;
                m.opponent_ids.push(player);
                m.base.follow_target = Some(player);
                m.base.attack_target = Some(player);
                m.state = MonsterState::Attacking;
                m.chase_mode = MonsterChaseMode::Close;
            }
        }
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(follower) {
            m.base.todo.queue.clear();
            m.base.walk_queue.clear();
            m.base.next_wakeup = None;
            m.base.has_follow_path = false;
        }

        world.map.register_creature_at(apos, blocker);
        world.map.unregister_creature_at(apos, blocker);
        world.map.register_creature_at(apos_moved, blocker);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(blocker) {
            m.base.position = apos_moved;
        }
        world.monster_dispatch_creature_move(blocker, apos, apos_moved);

        let base = world.creatures.get(follower).unwrap().base();
        assert!(
            base.todo.has_go() || base.next_wakeup.is_some() || !base.walk_queue.is_empty(),
            "stalled follower must re-arm chase when a blocking monster moves"
        );
    }

    fn monster_is_parked(world: &GameWorld, cid: CreatureId) -> bool {
        world.creatures.get(cid).is_some_and(|k| {
            let base = k.base();
            base.attack_target.is_some()
                && base.todo.is_empty()
                && base.walk_queue.is_empty()
                && base.next_wakeup.is_none()
        })
    }

    /// LOS blocked at cheb>1 must still arm close-chase approach — not park on bound target.
    #[test]
    fn test_772_attacking_los_blocked_does_not_freeze() {
        use crate::tile::{flags as tilestate, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let wall = Position::new(101, 100, 7);
        let ppos = Position::new(103, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        for pos in [(100, 101), (101, 101), (102, 100), (102, 101), (103, 100)] {
            ensure_walkable_tile(&mut world.map, Position::new(pos.0, pos.1, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        world.map.insert_tile(
            wall,
            Tile::Normal(TileBody {
                ground: Some(TEST_SYNTHETIC_GROUND_WP),
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: tilestate::BLOCKSOLID | tilestate::BLOCKPATH | tilestate::UNTHROW,
                zone: ZoneType::Normal,
            }),
        );

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.base.has_follow_path = false;
            m.base.walk_queue.clear();
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        assert!(
            !world.map.is_sight_clear(mpos, ppos),
            "test setup must block LOS between monster and player"
        );

        world.monster_idle_stimulus(monster);

        assert!(
            !monster_is_parked(&world, monster),
            "ATTACKING monster with blocked LOS must still arm chase or roam, not park"
        );
    }

    /// Diverged follow/attack dest must sync and escalate to roam — not infinite Wait(200).
    #[test]
    fn test_772_close_chase_target_divergence_no_wait_loop() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        let decoy = Position::new(100, 103, 7);
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        ensure_walkable_tile(&mut world.map, decoy, TEST_SYNTHETIC_GROUND_WP);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        let decoy_player = insert_player(&mut world, test_player("Decoy", decoy));
        world.map.register_creature_at(ppos, player);
        world.map.register_creature_at(decoy, decoy_player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(decoy_player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        world.monster_idle_stimulus(monster);

        let base = world.creatures.get(monster).unwrap().base();
        assert_eq!(
            base.follow_target,
            Some(player),
            "Attacking idle must sync follow_target to attack_target"
        );
        assert!(
            !world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().todo.has_wait() && {
                    k.base()
                        .todo
                        .queue
                        .iter()
                        .any(|a| matches!(a, CreatureAction::Wait { delay_ms: 200 }))
                }),
            "diverged dest must not loop Wait(200) when off-band close chase fails"
        );
        assert!(
            !monster_is_parked(&world, monster),
            "must arm Go/roam or clear target — not park"
        );
    }

    /// ~1 Hz think rescues monsters parked on a live target with no scheduler state.
    #[test]
    fn test_772_parked_monster_rescued_by_think() {
        use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;

        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(103, 100, 7);
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.base.todo.queue.clear();
            m.base.walk_queue.clear();
            m.base.next_wakeup = None;
            m.base.has_follow_path = false;
        }
        assert!(monster_is_parked(&world, monster));

        world.add_creature_think_check(monster);
        world.monster_on_think(monster, EVENT_CREATURE_THINK_INTERVAL_MS);

        assert!(
            !monster_is_parked(&world, monster),
            "ProcessCreatures think must re-arm idle for parked combat monster"
        );
    }

    /// ATTACKING close-chase must enqueue at engagement range (cheb>8), not only strike band.
    #[test]
    fn test_772_attacking_close_chase_at_cheb11() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(111, 100, 7);
        for x in 100..=111u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Snake", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.is_hostile = true;
            m.melee_skill = 15;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::Close;
            m.base.todo.queue.clear();
            m.base.next_wakeup = None;
        }

        world.monster_idle_stimulus(monster);

        assert!(
            !monster_is_parked(&world, monster),
            "ATTACKING at cheb=11 must close-chase via attack tail, not park"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| { k.base().todo.has_go() || k.base().next_wakeup.is_some() }),
            "cheb=11 must enqueue attack-path Go"
        );
    }

    /// Attack execute `Skipped` must not leave Attack in todo without a wakeup (dead queue).
    #[test]
    fn test_772_attack_execute_skipped_reschedules_not_parks() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(102, 100, 7);
        for x in 100..=102u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let monster = insert_monster(&mut world, "Rat", mpos, 200);

        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.is_idle = false;
            m.melee_skill = 15;
            m.state = MonsterState::Attacking;
            m.chase_mode = MonsterChaseMode::None;
            m.opponent_ids.push(player);
            m.base.follow_target = Some(player);
            m.base.attack_target = Some(player);
            m.base.todo.queue.push_back(CreatureAction::Attack);
            m.base.next_wakeup = None;
        }

        world.run_monster_todo_execute(monster);

        let base = world.creatures.get(monster).unwrap().base();
        assert!(
            base.next_wakeup.is_some() || base.todo.has_go() || !base.todo.is_empty(),
            "Skipped close-chase must reschedule todo drain, not dead-queue park"
        );
        assert!(
            !monster_is_parked(&world, monster),
            "attack execute Skipped must not park"
        );
    }
}
