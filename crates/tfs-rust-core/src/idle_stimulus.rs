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
use tfs_rust_common::enums::{CombatType, ConditionType, SpeakType, ZoneType};
use tfs_rust_common::game_packet::ThrowPayload;
use tfs_rust_common::Position;

use crate::chase_debug;
use crate::combat::math::spell_damage;
use crate::combat::{CombatDamage, CombatParams};
use crate::condition::{ActiveCondition, ConditionData};
use crate::creature::{
    monster_weapon_attack_distance, CreatureBase, CreatureKind, ChaseMode, MonsterSpell,
    MonsterState, SpellImpact, SpellShape,
};
use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;
use crate::creature_todo::{trace_creature_todo, ActionObjectRef, CreatureAction, MONSTER_IDLE_WAIT_MS};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::return_value::ReturnValue;
use crate::monster_ai::{
    chebyshev, compute_look_toward_target, manhattan, monster_idle_chase_step_budget,
    monster_master_follow_in_wait_band, MonsterCombatCloseChaseEnqueue,
    MonsterEnqueueAttackResult, MonsterIdleChaseRepathOutcome,
};
use crate::monster_targets::TargetSearchType;
use crate::player_flags::{flags_for_group, has_player_flag, PLAYER_FLAG_IGNORED_BY_MONSTERS};
use crate::walk::creature_turn_with_broadcast;

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
    /// F8 S3 — generic `CalculateDelay` gate deferral (e.g. two-object `Use` waiting on
    /// `EarliestMultiuseTime`). The wakeup was already armed by `todo_start_from_action`
    /// in the gate check, so the post-execute handler is a no-op — mirrors C++ `Execute`'s
    /// "Delay > 0 → schedule + break" (`cract.cc:795-801`).
    Deferred,
}

impl GameWorld {
    /// 772 `TCreature::IdleStimulus` — dispatch on creature kind.
    ///
    /// Phase 1 walk-engine unification: widened to dispatch **players** to
    /// [`player_idle_stimulus`] (`crplayer.cc:388-405`) in addition to monsters
    /// (`crnonpl.cc:2386`). NPCs remain excluded.
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
            Some(CreatureKind::Player(_)) => {
                trace_creature_todo(self, cid, "idle_stimulus_enter");
                self.player_idle_stimulus(cid);
                trace_creature_todo(self, cid, "idle_stimulus_exit");
            }
            _ => {}
        }
    }

    /// 772 `TPlayer::IdleStimulus` — `crplayer.cc:388-405`.
    ///
    /// Player idle handles **only** `Combat.AttackDest`: `ToDoAttack` → `ToDoStart`. The thrown
    /// `RESULT` → `ToDoClear` + `SendResult` (unless `NOERROR`/`NOWAY`) + `ToDoWait(1000)` +
    /// `ToDoStart` path is handled in [`Self::player_execute_attack`] (the `TDAttack` execute
    /// arm + `CanToDoAttack` chase, Phase 1.4). There is **no** separate follow re-path — follow
    /// lives in Combat (`CanToDoAttack`).
    ///
    /// With no attack target the player simply goes idle — the ToDo queue is empty and no
    /// wakeup is armed. This is the key fix for audit P2/P6: players no longer re-arm via the
    /// old `walk_queue` + `add_event_walk` path.
    fn player_idle_stimulus(&mut self, cid: CreatureId) {
        let has_attack_target = self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().attack_target.is_some());
        if !has_attack_target {
            // No attack target → idle drain complete, no re-arm (`crplayer.cc:388-405`).
            return;
        }
        // `Combat.AttackDest` is set → `ToDoAttack(); ToDoStart();` (`crplayer.cc:392-395`).
        // The thrown `RESULT` catch lives in `player_execute_attack` (the `TDAttack` execute arm).
        let _ = self.enqueue_creature_attack(cid);
        let attack_delay = self.todo_attack_delay_ms(cid);
        let delay = if attack_delay > 0 { attack_delay } else { 1 };
        self.todo_start_from_action(cid, delay);
    }

    /// Request idle when the action queue is drained — sync or deferred to next wakeup.
    ///
    /// Phase 1: widened from monster-only to all creatures on the unified ToDo path
    /// (players + monsters) via [`creature_uses_todo_execute`](Self::creature_uses_todo_execute).
    pub(crate) fn request_idle_stimulus(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        if !self.creature_uses_todo_execute(cid) {
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
        // Monster-only dedupe: one `IdleStimulus` pass per beat (`crnonpl.cc:2345`).
        // Players have no equivalent throttle — `IdleStimulus` is only called on queue drain.
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

    /// 772 `Execute` catch `EXHAUSTED` recovery (`cract.cc:870-877`):
    /// `ToDoClear() + ToDoWait(1000) + ToDoStart()`. The `Execute` catch does NOT clear `Target`
    /// itself — it relies on the throw site:
    /// - player-tile (`crnonpl.cc:2236-2238`): `Target = 0` before `throw EXHAUSTED`
    /// - kick-kill (`crnonpl.cc:2241-2242`): `KickCreature` returned false → `throw EXHAUSTED`
    ///   (Target NOT cleared)
    ///
    /// `clear_target` mirrors this distinction. The previous implementation unconditionally
    /// cleared the target, citing the `IdleStimulus` catch (`crnonpl.cc:2890-2898`) — wrong catch
    /// block (audit F3).
    pub(crate) fn monster_exhausted_wait_772(&mut self, cid: CreatureId, clear_target: bool) {
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            if clear_target {
                base.clear_targets(); // player-tile only (`crnonpl.cc:2237`)
            }
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

            // 772 physical-hit blood: race-keyed effect + blood/slime splash on the victim's tile
            // (`TCreature::Damage`, `crmain.cc:762-775`). Emitted for any physical damage that
            // landed, including the killing blow (C++ emits the effect before `Kill()`); the
            // full-blood pool is added afterwards by the death path.
            if self.beat_driven_loop
                && stimulus_damage > 0
                && damage.primary.0 == CombatType::Physical
            {
                if let Some(pos) = self.creatures.get(target).map(|k| k.position()) {
                    self.apply_physical_hit_blood_772(target, pos);
                }
            }

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

    /// C++ `TMonster::CreatureMoveStimulus` sleep wake — `crnonpl.cc:2943-2982`.
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
        // C++ wake gate (`crnonpl.cc:2969-2975`): NPC → no; Monster → only if
        // `IsPlayerControlled()` (master is a PLAYER, `crnonpl.cc:3139-3146`);
        // Player → yes. Wild monsters and NPC-owned summons do NOT wake sleepers.
        // `is_summon()` alone is too broad (any master) — require a player master.
        let should_wake = match self.creatures.get(moved_id) {
            None => false,
            Some(CreatureKind::Npc(_)) => false,
            Some(CreatureKind::Player(_)) => true,
            Some(CreatureKind::Monster(m)) => m.base.master.is_some_and(|mid| {
                matches!(self.creatures.get(mid), Some(CreatureKind::Player(_)))
            }),
        };
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

    /// C++ target validity + `LoseTarget` — `crnonpl.cc:2368-2384`.
    /// 772 summon despawn / re-bind block — `crnonpl.cc:2359–2405`.
    ///
    /// Runs at the top of `IdleStimulus` for summons (`Master != 0`). Despawns when the master is
    /// gone, on a different floor (non-player master), or beyond 30 tiles / `|Δz| > 1`. Otherwise
    /// re-binds the summon's target: `Master->Combat.Following ? Target=0 : Target=Master->AttackDest`,
    /// falling back to `Target=Master` when that clears or points at self.
    ///
    /// Returns `true` when the summon was despawned (caller must early-return — the creature is gone).
    fn monster_idle_summon_lifecycle_772(&mut self, cid: CreatureId) -> bool {
        let (master_id, master_is_player, summon_pos) = match self.creatures.get(cid) {
            Some(k) => match k.base().master {
                Some(m) => (
                    m,
                    matches!(self.creatures.get(m), Some(CreatureKind::Player(_))),
                    k.position(),
                ),
                None => return false, // Not a summon — skip the block.
            },
            None => return false,
        };

        let master_present = self.creatures.contains_key(master_id);
        let should_despawn = if !master_present {
            // C++ `Master == NULL` → despawn (`crnonpl.cc:2363`).
            tracing::debug!(?cid, ?master_id, master_is_player, "summon despawn: master gone");
            true
        } else {
            let master_pos = self
                .creatures
                .get(master_id)
                .map(|k| k.position())
                .unwrap_or(summon_pos);
            // C++ non-player master on a different floor → despawn (`crnonpl.cc:2373`).
            if !master_is_player && master_pos.z != summon_pos.z {
                tracing::debug!(?cid, ?master_id, "summon despawn: monster master on different floor");
                true
            } else {
                // C++ `|Δz| > 1 || |Δx| > 30 || |Δy| > 30` → despawn (`crnonpl.cc:2376`).
                let dz = (master_pos.z as i32 - summon_pos.z as i32).unsigned_abs();
                let dx = (master_pos.x as i32 - summon_pos.x as i32).unsigned_abs();
                let dy = (master_pos.y as i32 - summon_pos.y as i32).unsigned_abs();
                if dz > 1 || dx > 30 || dy > 30 {
                    tracing::debug!(?cid, ?master_id, dz, dx, dy, "summon despawn: too far from master");
                    true
                } else {
                    false
                }
            }
        };

        if should_despawn {
            // C++ player master → `StartLogout(true, true)`; monster master → `Kill()` (`crnonpl.cc:2388`).
            // Both paths set `State = SLEEPING` and return. Rust `remove_creature` covers the
            // disappear broadcast + summon-chain cleanup; `apply_creature_death` is reserved for
            // combat kills (loot/XP), not lifecycle despawns.
            self.remove_creature(cid);
            return true;
        }

        // Re-bind — `crnonpl.cc:2397–2405`. `Combat.Following` maps to an active follow target on
        // the master; `Combat.AttackDest` is the master's attack target.
        let master_following = self
            .creatures
            .get(master_id)
            .is_some_and(|k| k.base().follow_target.is_some());
        let master_attack_dest = self
            .creatures
            .get(master_id)
            .and_then(|k| k.base().attack_target);

        let new_target = if master_following {
            None // `Target = 0`
        } else {
            master_attack_dest // `Target = Master->Combat.AttackDest`
        };
        // C++ `if (Target == 0 || Target == self) Target = Master` (`crnonpl.cc:2403`).
        let new_target = match new_target {
            Some(t) if t != cid => Some(t),
            _ => Some(master_id),
        };

        // Apply via the existing target helpers so follow/attack stay aligned. The later
        // `monster_think_summon_stub` pass refines the follow target; this block sets the
        // authoritative attack-dest per C++.
        if let Some(target_id) = new_target {
            if self.monster_is_target(cid, target_id) {
                let _ = self.monster_set_follow_creature(cid, Some(target_id));
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().attack_target = Some(target_id);
                }
            } else {
                // Target not in opponent list (e.g. master itself) — still set follow per C++.
                let _ = self.monster_set_follow_creature(cid, Some(target_id));
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().attack_target = Some(target_id);
                }
            }
        } else {
            let _ = self.monster_set_follow_creature(cid, None);
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().attack_target = None;
            }
        }

        false
    }

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
            // C++ `crnonpl.cc:2426` `IsProtectionZone(Target->posx, …)`.
            if tile.body().zone == ZoneType::Protection {
                return true;
            }
            // C++ `crnonpl.cc:2427` `IsHouse(Target->posx, Target->posy, Target->posz)`
            // (AI#25). House tiles are a hard lose-target — monsters don't chase into houses.
            if matches!(tile, crate::tile::Tile::House(_)) {
                return true;
            }
        }
        // C++ `crnonpl.cc:2429` `(Target->IsInvisible() && !RaceData[this->Race].SeeInvisible)`
        // (AI#25). Monsters without `SeeInvisible` lose invisible targets.
        if target.base().is_invisible() && !m.see_invisible {
            return true;
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
        let Some((pos, existing_follow, _state, strat_near, strat_hp, strat_dmg)) = snapshot else {
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
                    .map(|k| k.base().damage_map.get(target_id).copied().unwrap_or(0) as i32)
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
            // C++ `VictimShapeSpell` (`magic.cc:423`) and `CircleShapeSpell` (used by
            // `DestinationShapeSpell`, `magic.cc:522`) both check `Actor->posz != DestZ`
            // → return. `OriginShapeSpell`/`AngleShapeSpell` use `Actor->posz` for all
            // tiles (implicitly same-Z). `Actor` is self-cast. So only `Victim` and
            // `Destination` need the gate — `Origin`/`Angle` tiles already use `caster_pos.z`
            // (`monster_idle_spell_tiles` lines 684, 701).
            if matches!(spell.shape, SpellShape::Victim | SpellShape::Destination)
                && pos.z != target_pos.z
            {
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

        // C++ summon despawn / re-bind block — runs at the very top of `IdleStimulus`
        // (`crnonpl.cc:2359–2405`), BEFORE the sleeping/idle checks. A sleeping summon still
        // gets despawned if its master is gone / too far / on a different floor.
        if self.beat_driven_loop && is_summon && self.monster_idle_summon_lifecycle_772(cid) {
            return;
        }

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

        // RC2: C++ `IdleStimulus` idle-wandering catch-all — `crnonpl.cc:2920–2939`.
        // Every path through the C++ function either returns early with `ToDoStart()` (combat
        // branches) or falls through to the idle-wandering tail which always ends with
        // `ToDoWait(1000) + ToDoStart()`. The decomposed Rust helpers above cover the combat
        // branches and the chase-target parked case, but when no target exists and roam fails
        // (Hold outcome), no wakeup was scheduled — causing ~750 ms stalls. This tail mirrors
        // the C++ catch-all: if nothing above armed a wakeup, enqueue a 1000 ms re-think.
        let already_armed = self.creatures.get(cid).is_some_and(|k| {
            let base = k.base();
            !base.todo.is_empty() || base.next_wakeup.is_some()
        });
        if !already_armed {
            self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
        }
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
                m.base.chase_mode,
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

    /// C++ talk gate + broadcast — `crnonpl.cc:2442–2458`:
    /// `if (Talks > 0 && (rand() % 50) == 0)` → `TalkNr = random(1, Talks)` → fetch text →
    /// `Talk(this->ID, Mode, NULL, Text, false)`. `Mode = TALK_ANIMAL_LOW`; a `#y `/`#Y ` prefix
    /// (decompile) or `<voice yell="1">` (TVP) switches to `TALK_ANIMAL_LOUD` and strips the prefix.
    ///
    /// Wire speak types (TVP `gameserver/src/const.h`): `TALKTYPE_MONSTER_YELL = 0x10`,
    /// `TALKTYPE_MONSTER_SAY = 0x11`. The RNG draw order (gate then pick) is preserved exactly so
    /// the glibc parity stream stays aligned with the sim harness.
    fn monster_idle_try_talk(&mut self, cid: CreatureId) {
        // Borrow the talk list + count together so we don't hold a borrow across the RNG draws.
        let (talks, talk_texts) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.talks, m.talk_texts.clone()),
            _ => return,
        };
        if talks == 0 || talk_texts.is_empty() {
            return;
        }
        let _trace_gate = crate::sim_glibc_rand::sim_rng_trace_site("idle_talk_gate");
        if self.parity_rand_mod(50) != 0 {
            return;
        }
        let _trace_pick = crate::sim_glibc_rand::sim_rng_trace_site("idle_talk_pick");
        // C++ `TalkNr = random(1, Talks)` — 1-indexed; Rust `talk_texts` is 0-indexed.
        let talk_nr = self.parity_random(1, i32::from(talks));
        let idx = (talk_nr.max(1) as usize).saturating_sub(1).min(talk_texts.len() - 1);
        let raw = &talk_texts[idx];

        // C++ `if (Text[0] == '#' && Text[1] != 0 && Text[2] == ' ')` yell marker (`crnonpl.cc:2450`).
        // TVP equivalent: `<voice yell="1">` sets `voiceBlock.yellText` (`monster.cpp:851`).
        // We support the decompile `#y `/`#Y ` prefix in the sentence text.
        const TALKTYPE_MONSTER_SAY: u8 = 0x11;
        const TALKTYPE_MONSTER_YELL: u8 = 0x10;
        let (speak_type, text) = if raw.len() >= 3
            && raw.as_bytes()[0] == b'#'
            && (raw.as_bytes()[1] == b'y' || raw.as_bytes()[1] == b'Y')
            && raw.as_bytes()[2] == b' '
        {
            (TALKTYPE_MONSTER_YELL, &raw[3..])
        } else {
            (TALKTYPE_MONSTER_SAY, raw.as_str())
        };

        // C++ `if (Text != 0 && Text[0] != 0)` — skip empty text after prefix strip.
        if text.is_empty() {
            return;
        }
        self.broadcast_creature_say_viewport(cid, speak_type, text);
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
            ChaseMode::None
        } else if let Some(follow_id) = follow_id {
            let target_distance = self.monster_effective_target_distance(raw_target_distance);
            let uses_dist_branch =
                self.monster_idle_uses_dist_branch(cid, pos, follow_id, target_distance);
            if uses_dist_branch {
                ChaseMode::None
            } else {
                ChaseMode::Close
            }
        } else {
            ChaseMode::None
        };
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.base.chase_mode = new_mode;
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
            let trace_key = (m.state, m.base.chase_mode);
            if m.last_combat_trace == Some(trace_key) {
                return None;
            }
            m.last_combat_trace = Some(trace_key);
            let mode = match m.base.chase_mode {
                ChaseMode::Close => "close",
                ChaseMode::Range => "range",
                ChaseMode::None => "none",
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
            // C++ `ToDoStart` always arms `NextWakeup` for the head todo entry when the list is
            // non-empty (`cract.cc:1010-1023`). The walk branch normally schedules the Go via
            // `idle_enqueue_paced_go`, but when `skip_idle_melee_chase` is true (ATTACKING/PANIC
            // at dist>1) the walk branch is `Hold` and the Go is enqueued here by
            // `monster_combat_enqueue_close_chase_go` — without a wakeup. That left the monster
            // parked with `[Go, Attack]` and no heap entry until the ~1 Hz think tick rescued it
            // via `monster_combat_reschedule_if_stalled`, causing a visible ~1 s stall after every
            // chase-batch drain while the target was kiting (audit: close-chase-wakeup-gap).
            let needs_wakeup = self
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().next_wakeup.is_none());
            if needs_wakeup {
                let has_go = self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| k.base().todo.has_go());
                if has_go {
                    // Go was enqueued by close-chase — schedule its walk delay (head entry).
                    let _ = self.todo_start_go_delay(cid, true);
                } else {
                    let delay_ms = self.todo_attack_delay_ms(cid);
                    self.todo_start_from_action(cid, delay_ms);
                }
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
        let _dist = chebyshev(pos, target_pos);
        // `CanToDoAttack` close walk at cheb>1 — no strike-range cap (`crcombat.cc:496`).
        if m.melee_skill > 0 && m.base.chase_mode == ChaseMode::Close {
            return true;
        }
        if target_distance <= 1 {
            return true;
        }
        self.monster_can_use_attack(cid, pos, attack_id)
    }

    /// C++ `Rotate(Target)` at idle combat tail — `crnonpl.cc:2871` (after `ToDoGo`, before
    /// `ToDoAttack`). Called **directly** (not enqueued), matching the C++ unconditional
    /// `Rotate(Target)` direct call. The 0x6B turn broadcast and the first `TDGo` move packet
    /// land in the same beat, so the client renders the turn imperceptibly — the move packet
    /// immediately overrides the facing direction. Enqueuing `Rotate` as a todo action (the
    /// Phase 8 approach) caused a visible "turn on the spot" because the 0x6B fired in a
    /// separate beat from any move packet (audit: turn-on-spot defect).
    pub(crate) fn monster_idle_rotate_toward_attack_target(&mut self, cid: CreatureId) {
        if !self.beat_driven_loop {
            return;
        }
        let target_id = self.creatures.get(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            if !matches!(m.state, MonsterState::Attacking | MonsterState::Panic) {
                return None;
            }
            m.base.attack_target
        });
        if let Some(target_id) = target_id {
            self.monster_execute_rotate_toward(cid, target_id);
        }
    }

    /// C++ `Rotate(TCreature *Target)` — `cract.cc:452-473`. No `walk_timer_idle` gate: the
    /// turn is unconditional, matching the C++ idle tail (`crnonpl.cc:2872-2873`). If the
    /// target is gone, no-op (C++ checks `Target == NULL` and returns).
    pub(crate) fn monster_execute_rotate_toward(&mut self, cid: CreatureId, target_id: CreatureId) {
        let (pos, current) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.base.position, m.base.direction),
            _ => return,
        };
        let target_pos = match self.creatures.get(target_id) {
            Some(k) => k.position(),
            None => return,
        };
        let new_dir = compute_look_toward_target(pos, target_pos, current);
        if new_dir != current {
            creature_turn_with_broadcast(self, cid, new_dir);
            if chase_debug::chase_path_debug_enabled() {
                if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                    chase_debug::log_rotate(
                        self.chase_trace_tick(),
                        cid,
                        m.base.name.as_str(),
                        new_dir as u8,
                        Some(target_id.data().as_ffi()),
                    );
                }
            }
        }
    }

    /// 772 NOWAY fall-through — clear chase target and roam (`crnonpl.cc:2890-2898` + `:2900-2939`).
    ///
    /// Mirrors the C++ `catch(RESULT r)` block in `TMonster::IdleStimulus`: when the close-chase
    /// `ToDoGo` (via `CanToDoAttack`) throws NOWAY because `TShortway::Calculate` found no path,
    /// C++ clears `Target`, `ToDoClear()`, and — for NOWAY (non-EXHAUSTED) — falls through to the
    /// idle-wandering roam tail (`crnonpl.cc:2900-2939`). EXHAUSTED (kick-kill / player-tile) is
    /// handled separately by the walk executor (`monster_exhausted_wait_772`).
    ///
    /// Used by the attack-tail NOWAY arm ([`Self::monster_idle_maybe_enqueue_attack`]) so an
    /// ATTACKING melee monster with no path to the target clears its target and roams instead of
    /// parking indefinitely. The walk-branch NOWAY handler
    /// ([`Self::monster_idle_prepare_and_enqueue_go`]) inlines the same clear+roam via its own
    /// `match outcome` block.
    pub(crate) fn monster_idle_noway_clear_and_roam(&mut self, cid: CreatureId) {
        self.monster_on_chase_noway_772(cid);
        let outcome = self.monster_idle_execute_walk_branch(cid, MonsterIdleWalkBranch::Roam);
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
            // Roam found no walkable tile — C++ `ToDoWait(1000) + ToDoStart()`
            // (`crnonpl.cc:2937-2939`). The idle catch-all (`crnonpl.cc:2920-2939` tail) also
            // covers this, but arming the wait here keeps the contract explicit.
            MonsterIdleWalkOutcome::Hold | MonsterIdleWalkOutcome::Noway => {
                self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
            }
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
                // C++ `catch(RESULT r)` for NOWAY: clear `Target` + fall through to roam
                // (`crnonpl.cc:2890-2898` + `:2900-2939`). Was: recurse into
                // `monster_idle_prepare_and_enqueue_go` which, with the target still set and
                // state ATTACKING, re-entered the same Hold→close-chase→Noway path — an
                // infinite recursion / parking loop.
                self.monster_idle_noway_clear_and_roam(cid);
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
            let k = self.creatures.get_mut(cid)?;
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
            CreatureAction::Talk { text } => {
                // C++ `TDTalk` — `cract.cc:848-851`, `:1367-1390`: `this->Talk(Mode, NULL, Text, false)`.
                // Talk mode: `TALK_SAY` for players, `TALK_ANIMAL_LOW` for monsters (`cract.cc:409`).
                trace_creature_todo(self, cid, "execute_talk");
                let is_monster = self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(_)));
                let speak_type = if is_monster {
                    SpeakType::MonsterSay as u8
                } else {
                    SpeakType::Say as u8
                };
                self.broadcast_creature_say_viewport(cid, speak_type, text);
                trace_creature_todo(self, cid, "execute_talk_done");
                TodoExecuteKind::Wait
            }
            CreatureAction::Attack => {
                // Phase 1.4: player Attack execute routes through `CanToDoAttack` chase
                // (`crcombat.cc:442-511`). The melee **strike** is deferred until a player
                // weapon-combat system exists; the chase (`ToDoGo` toward target) and the
                // thrown-`RESULT` `ToDoClear` + `SendResult` + `ToDoWait(1000)` path land here
                // (`crplayer.cc:388-405`, `cract.cc:870-889`).
                let is_player = self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| matches!(k, CreatureKind::Player(_)));
                if is_player {
                    self.player_execute_attack(cid)
                } else {
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
            }
            // F8 S4 — `Use`/`Move`/`Turn` execute arms. S3 multiuse gate still runs first
            // (two-object `Use` deferral); the executor dispatch + `RESULT` catch land here.
            // F8 S5 — not-adjacent Use/Move prepend `Go` + re-enqueue instead of the bespoke
            // `walk_action_due` path (C++ `Use` executor `cract.cc:600-760`).
            CreatureAction::Use { obj1, obj2, open_index } => {
                // F8 S3 — C++ `CalculateDelay(TDUse)` gate (`cract.cc:925-932`): two-object
                // use defers when `EarliestMultiuseTime > ServerMilliseconds`; single-object
                // use is ungated (delay 0). The action was already popped at the top of
                // `execute_creature_todo_action`, so we pass `obj2.is_some()` directly to
                // `multiuse_gate_delay_ms`. On deferral we push it back to the front and
                // arm a wakeup at `earliest_multiuse_server_ms` — same pattern as the
                // `Attack` defer above (`cract.cc:870-889`, `:795-801`).
                let delay = self.multiuse_gate_delay_ms(cid, obj2.is_some());
                if delay > 0 {
                    if let Some(k) = self.creatures.get_mut(cid) {
                        k.base_mut().todo.queue.push_front(CreatureAction::Use {
                            obj1,
                            obj2,
                            open_index,
                        });
                    }
                    trace_creature_todo(self, cid, "execute_use_deferred");
                    self.todo_start_from_action(cid, delay);
                    trace_creature_todo(self, cid, "execute_use_deferred_done");
                    TodoExecuteKind::Deferred
                } else {
                    // F8 S5 — walk-to-reach via `Go`-prepend (C++ `Use` executor
                    // `cract.cc:600-760`): if obj1 is a map tile and the player isn't
                    // adjacent, prepend `Go` + re-enqueue `Use` + `ToDoStart`. This
                    // replaces the bespoke `walk_action_due` path for the ToDo flow.
                    let needs_walk = obj1.pos.x != 0xFFFF
                        && self.creatures.get(cid).is_some_and(|k| {
                            crate::item_look::look_distance_tfs(k.position(), obj1.pos) > 1
                        });
                    if needs_walk {
                        let now = Instant::now();
                        match self.setup_player_walk_to_target(cid, obj1.pos, now) {
                            Ok(()) => {
                                // `push_front(Use)` then `push_front(Go)` → `[Go, Use, ...]`
                                // (C++ `ToDoGo(dest)` + re-enqueue `ToDoUse`, `cract.cc:600-760`).
                                if let Some(k) = self.creatures.get_mut(cid) {
                                    k.base_mut().todo.queue.push_front(CreatureAction::Use {
                                        obj1,
                                        obj2,
                                        open_index,
                                    });
                                    k.base_mut().todo.queue.push_front(CreatureAction::Go);
                                }
                                if self.todo_start_go_delay(cid, true) {
                                    self.schedule_immediate_todo_wakeup(cid);
                                }
                                trace_creature_todo(self, cid, "execute_use_walk_to_reach");
                                TodoExecuteKind::Deferred
                            }
                            Err(rv) => {
                                self.apply_todo_result_catch(cid, rv);
                                TodoExecuteKind::Wait
                            }
                        }
                    } else {
                        // F8 S4 — `TDUse` execute (`cract.cc:833-836`). Re-validate the
                        // object (mirrors C++ `Obj.exists()` in the executor), then dispatch
                        // to `player_use_item_core` / `player_use_item_ex_core` (S5: core
                        // helpers skip the ready check + walk-to-reach — the ToDo arm handles
                        // adjacency via `Go`-prepend and timing via `Wait{100}` +
                        // `CalculateDelay`). On `Err(rv)` apply the C++ `RESULT` catch
                        // (`cract.cc:870-889`). Multiuse exhaustion is set inside
                        // `player_use_item_ex_core` on two-object success (`cract.cc:765`).
                        trace_creature_todo(self, cid, "execute_use");
                        let result = self.execute_player_use(cid, obj1, obj2, open_index);
                        if let Err(rv) = result {
                            self.apply_todo_result_catch(cid, rv);
                        }
                        trace_creature_todo(self, cid, "execute_use_done");
                        TodoExecuteKind::Wait
                    }
                }
            }
            CreatureAction::Move { obj, dest, count } => {
                // F8 S5 — walk-to-reach via `Go`-prepend. For map-tile sources, if the
                // player isn't within 1 tile of the source, prepend `Go` + re-enqueue
                // `Move` + `ToDoStart`. C++ `Move` executor re-validates the object at
                // execute time (`Obj.exists()` → `NOTACCESSIBLE`); the throw-destination
                // range check runs inside `player_move_item` after the walk (matches C++
                // behavior — walk there, then fail if dest unreachable). The reactive
                // path's `throw_dest_reachable_after_walk_to_item` pre-check stays in
                // `player_move_item` for the reactive caller; on the ToDo path it's a
                // no-op (player is adjacent after the walk, so `dx <= 1 && dy <= 1`).
                let needs_walk = obj.pos.x != 0xFFFF
                    && self.creatures.get(cid).is_some_and(|k| {
                        let pp = k.position();
                        let dx = (pp.x as i32 - obj.pos.x as i32).unsigned_abs();
                        let dy = (pp.y as i32 - obj.pos.y as i32).unsigned_abs();
                        dx > 1 || dy > 1
                    });
                if needs_walk {
                    let now = Instant::now();
                    match self.setup_player_walk_to_target(cid, obj.pos, now) {
                        Ok(()) => {
                            if let Some(k) = self.creatures.get_mut(cid) {
                                k.base_mut().todo.queue.push_front(CreatureAction::Move {
                                    obj,
                                    dest,
                                    count,
                                });
                                k.base_mut().todo.queue.push_front(CreatureAction::Go);
                            }
                            if self.todo_start_go_delay(cid, true) {
                                self.schedule_immediate_todo_wakeup(cid);
                            }
                            trace_creature_todo(self, cid, "execute_move_walk_to_reach");
                            TodoExecuteKind::Deferred
                        }
                        Err(rv) => {
                            self.apply_todo_result_catch(cid, rv);
                            TodoExecuteKind::Wait
                        }
                    }
                } else {
                    // F8 S4 — `TDMove` execute (`cract.cc:823-826`). C++ `CalculateDelay`
                    // `default` case: delay 0 (`cract.cc:946-948`); no gate. Re-validate the
                    // object, then dispatch to `player_move_thing` (reroute only — already
                    // exists, F8 §0.1 F5). On `Err(rv)` apply the `RESULT` catch.
                    trace_creature_todo(self, cid, "execute_move");
                    let result = self.execute_player_move(cid, obj, dest, count);
                    if let Err(rv) = result {
                        self.apply_todo_result_catch(cid, rv);
                    }
                    trace_creature_todo(self, cid, "execute_move_done");
                    TodoExecuteKind::Wait
                }
            }
            CreatureAction::Turn { obj } => {
                // F8 S4 — `TDTurn` execute (`cract.cc:838-841`). C++ `CalculateDelay`
                // `default` case: delay 0 (`cract.cc:946-948`); no gate.
                //
                // F8 D3 — walk-to-reach via `Go`-prepend (C++ `ToDoTurn` `cract.cc:1340-1341`:
                // `if(!ObjectInRange(this->ID, Obj, 1)) this->ToDoGo(ObjX, ObjY, ObjZ, false,
                // INT_MAX)`). For map-tile sources, if the player isn't within 1 tile,
                // prepend `Go` + re-enqueue `Turn` + `ToDoStart` — same shape as the `Use`/
                // `Move` S5 arms. The reach predicate uses the same-z Chebyshev `dx>1 ||
                // dy>1` form as the `Move` arm (matches `ObjectInRange(1)` for same-z; the
                // Δz case is D2/D6, handled separately). On `Err(rv)` from
                // `setup_player_walk_to_target` apply the C++ `RESULT` catch
                // (`cract.cc:870-889`).
                let needs_walk = obj.pos.x != 0xFFFF
                    && self.creatures.get(cid).is_some_and(|k| {
                        let pp = k.position();
                        let dx = (pp.x as i32 - obj.pos.x as i32).unsigned_abs();
                        let dy = (pp.y as i32 - obj.pos.y as i32).unsigned_abs();
                        dx > 1 || dy > 1
                    });
                if needs_walk {
                    let now = Instant::now();
                    match self.setup_player_walk_to_target(cid, obj.pos, now) {
                        Ok(()) => {
                            // `push_front(Turn)` then `push_front(Go)` → `[Go, Turn, ...]`
                            // (C++ `ToDoGo(dest)` + re-enqueue `ToDoTurn`, `cract.cc:1341`).
                            if let Some(k) = self.creatures.get_mut(cid) {
                                k.base_mut().todo.queue.push_front(CreatureAction::Turn { obj });
                                k.base_mut().todo.queue.push_front(CreatureAction::Go);
                            }
                            if self.todo_start_go_delay(cid, true) {
                                self.schedule_immediate_todo_wakeup(cid);
                            }
                            trace_creature_todo(self, cid, "execute_turn_walk_to_reach");
                            TodoExecuteKind::Deferred
                        }
                        Err(rv) => {
                            self.apply_todo_result_catch(cid, rv);
                            TodoExecuteKind::Wait
                        }
                    }
                } else {
                    // Adjacent (or inventory/container source) — dispatch to the
                    // `player_rotate_item` executor (F8 §0.1 F2 — nothing existed to
                    // reuse). On `Err(rv)` apply the `RESULT` catch.
                    trace_creature_todo(self, cid, "execute_turn");
                    let result = self.player_rotate_item(cid, obj);
                    if let Err(rv) = result {
                        self.apply_todo_result_catch(cid, rv);
                    }
                    trace_creature_todo(self, cid, "execute_turn_done");
                    TodoExecuteKind::Wait
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

    /// F8 S4/S5 — `TDUse` execute dispatch. Re-validates the object(s) at execute time
    /// (mirrors C++ `Obj.exists()` in the `Use` executor, `cract.cc:727-760`), resolves
    /// the `ItemId` from the `ActionObjectRef`, then calls the core use helpers
    /// (`player_use_item_core` / `player_use_item_ex_core`) directly — **skipping** the
    /// reactive-path ready check + walk-to-reach (S5: the ToDo arm handles adjacency via
    /// `Go`-prepend and timing via `Wait{100}` + `CalculateDelay`). Returns `Err(rv)` on
    /// re-validation or executor failure so the caller can apply the `RESULT` catch.
    pub(crate) fn execute_player_use(
        &mut self,
        cid: CreatureId,
        obj1: ActionObjectRef,
        obj2: Option<ActionObjectRef>,
        open_index: u8,
    ) -> Result<(), ReturnValue> {
        // Re-validate obj1 (and obj2 if present) — `validate_action_object_ref` resolves
        // the item + checks the sprite, returning `Err(NotPossible)` on mismatch (C++
        // `NOTACCESSIBLE` → `NotPossible`, `walk/mod.rs:1506` convention).
        self.validate_action_object_ref(cid, obj1)?;
        if let Some(o2) = obj2 {
            self.validate_action_object_ref(cid, o2)?;
        }

        let Some(conn_id) = self.conn_for_creature(cid) else {
            // Player disconnected — no conn to send results/open containers to.
            tracing::debug!(?cid, "execute_player_use: no conn — skipping");
            return Ok(());
        };

        // Resolve `ItemId` for obj1 — same resolution path as `validate_action_object_ref`
        // (`resolve_item_at_position` + `find_tile_item_by_client_sprite` fallback).
        let is_map_tile = obj1.pos.x != 0xFFFF;
        let item_id = if let Some(id) = self.resolve_item_at_position(cid, obj1.pos, obj1.stack_pos)
        {
            Some(id)
        } else if is_map_tile {
            self.find_tile_item_by_client_sprite(obj1.pos, obj1.sprite_id)
        } else {
            None
        };
        let Some(item_id) = item_id else {
            return Err(ReturnValue::NotPossible);
        };

        if obj2.is_some() {
            // Two-object use — `CUseTwoObjects` (`receiving.cc:430`). Core helper sets
            // multiuse exhaustion on success (`cract.cc:765`).
            self.player_use_item_ex_core(conn_id, cid, item_id)
        } else {
            // Single-object use — `CUseObject` (`receiving.cc:384`).
            let preferred_cid =
                (open_index < crate::container::MAX_CONTAINER_WINDOWS).then_some(open_index);
            self.player_use_item_core(conn_id, cid, item_id, is_map_tile, obj1.pos, preferred_cid)
        }
    }

    /// F8 S4 — `TDMove` execute dispatch. Re-validates the source object, reconstructs
    /// the `ThrowPayload` from the `ActionObjectRef` + destination, and calls the
    /// existing `player_move_thing` executor (reroute only — F8 §0.1 F5). Returns
    /// `Err(rv)` on re-validation or executor failure for the `RESULT` catch.
    pub(crate) fn execute_player_move(
        &mut self,
        cid: CreatureId,
        obj: ActionObjectRef,
        dest: Position,
        count: u8,
    ) -> Result<(), ReturnValue> {
        self.validate_move_object_ref(cid, obj)?;

        let Some(conn_id) = self.conn_for_creature(cid) else {
            tracing::debug!(?cid, "execute_player_move: no conn — skipping");
            return Ok(());
        };
        let now = Instant::now();
        let payload = ThrowPayload {
            from_pos: obj.pos,
            sprite_id: obj.sprite_id,
            from_stack_pos: obj.stack_pos,
            to_pos: dest,
            count,
        };
        self.player_move_thing(
            conn_id,
            cid,
            payload.from_pos,
            payload.sprite_id,
            payload.from_stack_pos,
            payload.to_pos,
            payload.count,
            now,
        )
    }

    /// Execute one `CreatureAction::Go` for 772 monsters — returns true if an action ran.
    #[cfg(test)]
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

        // C++ `Execute` checks `Stop` after a successful action (`cract.cc:891-897`) and when the
        // next step's `Delay > 0` (`cract.cc:797-801`): `ToDoClear + SendSnapback` (player only).
        // `todo_stop` is set by `player_stop_auto_walk` (772 `ToDoStop` locked branch,
        // `cract.cc:1003-1004`). The in-flight step has just landed; now clear + snapback.
        let stop_requested = self.creatures.get(cid).is_some_and(|k| {
            matches!(k, CreatureKind::Player(_)) && k.base().todo.todo_stop
        });
        if stop_requested {
            if let Some(conn) = self.conn_for_creature(cid) {
                let dir_byte = self
                    .creatures
                    .get(cid)
                    .map(|k| k.base().direction as u8)
                    .unwrap_or(0);
                self.enqueue_encoded(conn, self.codec.encode_cancel_walk(dir_byte));
            }
            self.player_todo_clear(cid);
            return;
        }

        let walk_queue_has_more = self
            .creatures
            .get(cid)
            .is_some_and(|k| !k.base().walk_queue.is_empty());

        if walk_queue_has_more {
            // `force_update_follow_path` is a monster chase-repath flag — only
            // monsters have a follow target and `IdleStimulus` to clear it.
            // For players, it must never gate step chaining (C++ `Execute` catch
            // does not set any follow-path flag — `cract.cc:870-889`).
            let force_repath = self
                .creatures
                .get(cid)
                .is_some_and(|k| {
                    matches!(k, CreatureKind::Monster(_)) && k.base().force_update_follow_path
                });
            if force_repath {
                if let Some(k) = self.creatures.get_mut(cid) {
                    let base = k.base_mut();
                    base.walk_queue.clear();
                    base.has_follow_path = false;
                }
                self.request_idle_stimulus(cid);
                return;
            }
            let queue_len = self
                .creatures
                .get(cid)
                .map(|k| k.base().walk_queue.len())
                .unwrap_or(0);
            // Re-arm `Go` before pending `Attack` — one step per execute (`cract.cc:728`).
            let _ = self.enqueue_creature_go_at(cid, true);
            let immediate = self.todo_start_go_delay(cid, false);
            tracing::debug!(
                ?cid,
                queue_len,
                immediate,
                server_ms = self.server_ms,
                "autowalk_772: finish_creature_todo_execute — chain next step"
            );
            if immediate {
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
    ///
    /// Phase 8 / GL#7: the atomic `Execute` drain is realized via the tail-recursion in
    /// [`finish_creature_todo_execute`] → `run_monster_todo_execute`, which chains zero-delay
    /// actions (e.g. `Rotate` → `Attack`) in one beat — semantically equivalent to C++'s
    /// `while(true)` `Execute` loop (`cract.cc:783-898`) and bounded by the `+1` re-insertion
    /// clamp (`ToDoStart`, audit Finding 17).
    pub(crate) fn run_monster_todo_execute(&mut self, cid: CreatureId) {
        match self.execute_creature_todo_action(cid) {
            Some(TodoExecuteKind::Go)
            | Some(TodoExecuteKind::Attack) => {
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
            // F8 S3 — gate-deferred action (two-object Use waiting on multiuse exhaustion).
            // The wakeup was already armed by `todo_start_from_action` in the gate check;
            // no reschedule needed (`cract.cc:795-801` "Delay > 0 → schedule + break").
            Some(TodoExecuteKind::Deferred) => {}
            None => {}
        }
    }
}


#[cfg(test)]
#[path = "idle_stimulus_tests.rs"]
mod tests;
