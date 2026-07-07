//! 772 player combat dest — `SetAttackDest` / `CanToDoAttack` / `StopAttack` chase routing.
//!
//! C++ reference (mechanics, `tibia-game-master/src/`):
//! - `TCombat::SetAttackDest` — `crcombat.cc:357-440`.
//! - `TCombat::CanToDoAttack` — `crcombat.cc:442-511`.
//! - `TCombat::StopAttack` — `crcombat.cc:513-522`.
//! - `CAttack` packet handler — `receiving.cc:1133-1155`.
//! - `TPlayer::IdleStimulus` thrown-`RESULT` path — `crplayer.cc:388-405`.
//!
//! Phase 1.4 walk-engine unification: routes 772 player attack/follow/cancel packets through the
//! unified ToDo engine. The **melee strike** (`TCombat::Attack` / `CloseAttack` /
//! `DistanceAttack` with player weapon damage) is **deferred** — no player weapon-combat system
//! exists yet. The chase (`CanToDoAttack` close walk) and target/clear-target wiring land here.
//!
//! 772 has **no** separate `follow_target` semantics — follow == attack-with-`Following`
//! (`crcombat.cc:493-495` sets `ChaseMode = CHASE_MODE_CLOSE` when `Following`). We still set
//! `follow_target` so the shared `Go`/pathfinding arms (which key off `follow_target`) repath
//! toward the target on the attack beat.

pub(crate) mod ranged;
pub(crate) mod strike;
pub(crate) mod values;

// Re-export `SkillNr` so downstream phases (PC-2+) can reference it as
// `crate::player::combat::SkillNr` without reaching into the `values` submodule.
// `#[allow(unused_imports)]` — no consumer until PC-2 wires `weapon_damage`/`defense_value`.
#[allow(unused_imports)]
pub(crate) use values::SkillNr;

use slotmap::Key;
use tfs_rust_common::enums::ZoneType;
use tfs_rust_common::ConnId;
use tfs_rust_net::outgoing_extra::send_text_message_simple;

use crate::creature::{CreatureKind, ChaseMode};
use crate::creature_todo::{trace_creature_todo, MONSTER_IDLE_WAIT_MS};
use crate::game_world::GameWorld;
use crate::idle_stimulus::TodoExecuteKind;
use crate::ids::CreatureId;
use crate::monster_ai::chebyshev;
use crate::return_value::ReturnValue;

/// C++ `RESULT` codes thrown by `SetAttackDest` / `CanToDoAttack` (`crcombat.cc`, `sending.cc:285`).
///
/// Mapped onto [`ReturnValue`] for the player-visible `SendResult` text (`ReturnValue::description`
/// mirrors `getReturnMessage`). `NoError` and `NoWay` suppress the `SendResult` text per
/// `crplayer.cc:395-398` (`if (r != NOERROR) { if (r != NOWAY) SendResult; ToDoWait(1000); }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatResult {
    NoError,
    /// `TARGETLOST` — target gone, dead, invisible (vs creature), or distance > 8.
    TargetLost,
    /// `PROTECTIONZONE` — master or target tile is a protection zone.
    ProtectionZone,
    /// `ATTACKNOTALLOWED` — target is an NPC, or master lacks attack rights.
    AttackNotAllowed,
    /// `SECUREMODE` — secure mode blocks attacking an unmarked player (PVP). Reserved for the
    /// player weapon-combat system (PVP `IsAttackJustified` check, `crcombat.cc:374-381`).
    #[allow(dead_code)]
    SecureMode,
    /// `OUTOFAMMO` — no ammo for bow, or insufficient mana for wand (`crcombat.cc:725,757`).
    /// 772 `sending.cc:348` `default: break` — no message sent; the catch path still does
    /// `ToDoWait(1000)` + `ToDoStart` (`crplayer.cc:395-401`).
    OutOfAmmo,
}

impl CombatResult {
    fn to_return_value(self) -> ReturnValue {
        match self {
            CombatResult::NoError => ReturnValue::NoError,
            CombatResult::TargetLost => ReturnValue::CreatureDoesNotExist,
            CombatResult::ProtectionZone => ReturnValue::ActionNotPermittedInProtectionZone,
            CombatResult::AttackNotAllowed => ReturnValue::YouMayNotAttackThisCreature,
            CombatResult::SecureMode => ReturnValue::TurnSecureModeToAttackUnmarkedPlayers,
            CombatResult::OutOfAmmo => ReturnValue::NotEnoughMana,
        }
    }

    fn is_noerror_or_noway(self) -> bool {
        matches!(self, CombatResult::NoError)
    }
}

impl GameWorld {
    /// 772 `TCombat::SetAttackDest` + `CAttack` packet body — `crcombat.cc:357-440`,
    /// `receiving.cc:1133-1155`.
    ///
    /// `follow = false` → `Attack` packet; `follow = true` → `Follow` packet. On success: set
    /// `attack_target` (+ `follow_target` when following), `enqueue_creature_attack` +
    /// `todo_start_from_action(attack_delay)`. On thrown `RESULT`: `ToDoClear` + `SendResult`
    /// (unless `NOERROR`/`NOWAY`) + `ToDoWait(1000)` + `ToDoStart` (`crplayer.cc:393-402`).
    ///
    /// Returns the `CombatResult` so the caller can decide whether to also `ToDoYield`
    /// (`receiving.cc:1149-1153` `CAttack` catch — `ToDoYield` only on non-`NOERROR`).
    pub(crate) fn player_set_attack_dest(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        target_wire_id: u32,
        follow: bool,
    ) -> CombatResult {
        // Phase 4: 1098 defer deleted — both eras use the 772 `SetAttackDest` ToDo path.
        // `SetAttackDest` early-out: same target + same follow → no-op (`crcombat.cc:358-360`).
        let resolved_target = target_wire_id_to_creature(self, target_wire_id);
        let already = self.creatures.get(cid).is_some_and(|k| {
            let b = k.base();
            b.attack_target == resolved_target && follow == b.follow_target.is_some()
        });
        if already {
            return CombatResult::NoError;
        }

        // `TargetID == 0` (cancel) or target self → `StopAttack` (`crcombat.cc:363-366`).
        if target_wire_id == 0 {
            self.player_stop_attack(conn_id, cid);
            return CombatResult::NoError;
        }
        let Some(target_id) = self.creature_by_wire_id(target_wire_id) else {
            self.player_stop_attack(conn_id, cid);
            return CombatResult::TargetLost;
        };
        if target_id == cid {
            self.player_stop_attack(conn_id, cid);
            return CombatResult::NoError;
        }

        // Validate — subset of `SetAttackDest` `!Follow` + universal checks. Secure-mode / PVP
        // `IsAttackJustified` is deferred to the player weapon-combat system.
        let result = self.validate_player_attack_target(cid, target_id);
        if result != CombatResult::NoError {
            self.player_stop_attack(conn_id, cid);
            // `CAttack` catch: `ToDoClear` + `SendResult` (unless NOERROR) + `ToDoYield`
            // (`receiving.cc:1149-1153`). `ToDoWait(1000)` is the idle-re-arm path (`crplayer.cc`).
            self.player_todo_clear(cid);
            if !result.is_noerror_or_noway() {
                self.send_combat_result(conn_id, result);
            }
            self.creature_todo_yield(cid);
            return result;
        }

        // Success — `AttackDest = TargetID; Following = Follow;` (`crcombat.cc:430-431`).
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.attack_target = Some(target_id);
            // 772 follow == attack-with-Following; set `follow_target` so shared `Go`/pathfinding
            // arms repath. `Following ⇒ ChaseMode = CHASE_MODE_CLOSE` (`crcombat.cc:493-495`).
            base.follow_target = follow.then_some(target_id);
            if follow {
                base.chase_mode = ChaseMode::Close;
            }
            // `LatestAttackTime = 0` so the first `Attack` execute isn't suppressed
            // (`crcombat.cc:438`).
            base.earliest_attack_ms = 0;
        }

        // `ToDoAttack()` → `ToDoAdd(TDAttack)`. Every `ToDoAdd` runs the `LockToDo` preamble:
        // when a walk/action is already armed, it `ToDoClear()`s the queue and — for a player
        // with a pending `Go` — `SendSnapback`s before appending the new entry
        // (`cract.cc:993-1000`, `ToDoAttack` `cract.cc:1353-1365`). Without this, issuing an
        // attack/follow mid-autowalk left the old `Go` + full `walk_queue` in place: the player
        // kept auto-walking the whole path (attack queued behind it) and the client never
        // resynced (no `0xB5`). The walk-request handlers already do this via
        // `player_todo_clear_with_snapback`; the attack handler was the missing override path.
        let lock_to_do = self.creatures.get(cid).is_some_and(|k| {
            let b = k.base();
            b.todo.locked || b.next_wakeup.is_some() || b.todo.has_go() || !b.walk_queue.is_empty()
        });
        if lock_to_do {
            self.player_todo_clear_with_snapback(conn_id, cid);
        }

        // `ToDoAttack(); ToDoStart();` (`receiving.cc:1147-1148`).
        let _ = self.enqueue_creature_attack(cid);
        let attack_delay = self.todo_attack_delay_ms(cid);
        let delay = if attack_delay > 0 { attack_delay } else { 1 };
        self.todo_start_from_action(cid, delay);
        trace_creature_todo(self, cid, "player_set_attack_dest");
        CombatResult::NoError
    }

    /// 772 `TCombat::StopAttack(0)` — `crcombat.cc:513-522`: clear `AttackDest` and, for players,
    /// `SendClearTarget` (`0xA3`).
    pub(crate) fn player_stop_attack(&mut self, conn_id: ConnId, cid: CreatureId) {
        let was_attacking = self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().attack_target.is_some());
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.attack_target = None;
            base.follow_target = None;
            // Reset to the chase mode last set by `FightModes` (Following forced CLOSE; clearing
            // follow restores the player's chosen mode). We do not store the pre-follow mode, so
            // leave `chase_mode` as-is — `CanToDoAttack` re-reads it next attack.
        }
        if was_attacking {
            self.enqueue_encoded(conn_id, self.codec.encode_clear_target());
            trace_creature_todo(self, cid, "player_stop_attack");
        }
    }

    /// 772 `CCancelAttack` — `receiving.cc:1330-1347`, `cract.cc:953-989`.
    ///
    /// `StopAttack` (→ `SendClearTarget`) + `ToDoClear` + `SendSnapback` if a pending `Go` was
    /// cleared (`receiving.cc:1338-1341`: `if(Player->ToDoClear()) SendSnapback`). Unlike
    /// `ToDoStop` (used by `CGoStop`), `CCancelAttack` calls `ToDoClear` directly — immediate
    /// clear + conditional snapback, no deferred `Stop` flag.
    pub(crate) fn player_cancel_attack_and_follow(&mut self, conn_id: ConnId, cid: CreatureId) {
        // Phase 4: 1098 defer deleted — both eras use the 772 `CCancelAttack` ToDo path.
        self.player_stop_attack(conn_id, cid);
        // `ToDoClear` + `SendSnapback` if pending Go (`receiving.cc:1339-1341`).
        self.player_todo_clear_with_snapback(conn_id, cid);
    }

    /// 772 `TCombat::CanToDoAttack` — `crcombat.cc:442-511`.
    ///
    /// Called from the player `Attack` execute arm. Returns whether a chase `Go` was armed (so the
    /// caller keeps the `Attack` deferred and re-arms on the `Go` wake) or the player is adjacent
    /// (strike range — but the strike itself is deferred; we just re-arm on the attack beat).
    pub(crate) fn player_can_to_do_attack_chase(&mut self, cid: CreatureId) -> PlayerChaseOutcome {
        let (target_id, pos, chase_mode, following) = match self.creatures.get(cid) {
            Some(k) => {
                let b = k.base();
                (
                    b.attack_target,
                    b.position,
                    b.chase_mode,
                    b.follow_target.is_some(),
                )
            }
            None => return PlayerChaseOutcome::NoTarget,
        };
        let Some(target_id) = target_id else {
            return PlayerChaseOutcome::NoTarget;
        };

        // `Target == NULL` → `StopAttack` + `TARGETLOST` (`crcombat.cc:454-458`).
        let target_pos = match self.creatures.get(target_id) {
            Some(k) if k.base().health > 0 => k.position(),
            _ => return PlayerChaseOutcome::TargetLost,
        };

        // `Distance > 8` → `StopAttack` + `TARGETLOST` (`crcombat.cc:486-490`). `ObjectDistance`
        // returns `INT_MAX` across Z-levels (`info.cc:313`), so cross-floor always drops here.
        if pos.z != target_pos.z {
            return PlayerChaseOutcome::TargetLost;
        }
        let cheb = chebyshev(pos, target_pos);
        if cheb > 8 {
            return PlayerChaseOutcome::TargetLost;
        }

        // `Following ⇒ ChaseMode = CHASE_MODE_CLOSE` (`crcombat.cc:493-495`).
        let effective_chase = if following { ChaseMode::Close } else { chase_mode };

        if effective_chase == ChaseMode::Close {
            if cheb > 1 {
                // `ToDoGo(target, false, 3)` — close chase (`crcombat.cc:498-499`). Reuse the generic
                // player-aware pathfinder (`get_creature_path_to` dispatches to
                // `tile_query_add_player` for the moving player).
                let Some(steps) = self.get_creature_path_to(cid, target_pos, 1, 1) else {
                    // No path — re-arm on the attack beat; `CanToDoAttack` will retry next tick.
                    return PlayerChaseOutcome::NoPath;
                };
                if steps.is_empty() {
                    return PlayerChaseOutcome::Adjacent;
                }
                if let Some(k) = self.creatures.get_mut(cid) {
                    let base = k.base_mut();
                    base.walk_queue.clear();
                    base.walk_destinations.clear();
                    // C++ `ToDoGo` stores absolute coordinates per `TDGo` entry (`cract.cc:1093-1095`,
                    // audit #4). `steps` is in forward execution order; `walk_queue` uses
                    // `push_back` in rev order + `pop_back` (LIFO) so `pop_back` yields the first
                    // step. Accumulate destinations in forward (execution) order and `push_front`
                    // so `pop_back` on both queues stays in sync.
                    for d in steps.iter().rev() {
                        base.walk_queue.push_back(*d);
                    }
                    let mut acc = pos;
                    for &d in &steps {
                        acc = acc.offset(d);
                        base.walk_destinations.push_front(acc);
                    }
                    base.has_follow_path = true;
                    base.force_update_follow_path = false;
                }
                // `ToDoGo` ahead of `TDAttack` (`cract.cc:1325` — `ToDoGo` then `TDAttack`).
                let _ = self.enqueue_creature_go_at(cid, true);
                if self.todo_start_go_delay(cid, false) {
                    self.schedule_immediate_todo_wakeup(cid);
                }
                trace_creature_todo(self, cid, "player_can_to_do_attack_chase");
                return PlayerChaseOutcome::ChaseArmed;
            }
            // Adjacent — strike range. Strike deferred; re-arm on the attack beat.
            PlayerChaseOutcome::Adjacent
        } else {
            // `CHASE_MODE_NONE` (or RANGE, which 772 players cannot set via `SetChaseMode`): no
            // chase walk. C++ `CanToDoAttack` does nothing, then `Attack()` checks the weapon
            // `Range` vs `Distance` (`crcombat.cc:611-639`).
            //
            // Ranged weapon (range ≥ 2): `Attack()` dispatches to `DistanceAttack`/`WandAttack`
            // when `Distance ≤ Range` and `abs(dx) ≤ 7 && abs(dy) ≤ 5` (`crcombat.cc:617-627`).
            // Otherwise `TARGETOUTOFRANGE` (re-arm, no message).
            //
            // Melee weapon (range 1): `Distance > 1` → `TARGETOUTOFRANGE` (`crcombat.cc:613-614`).
            let weapon_range = self.player_weapon_range(cid);
            if weapon_range >= 2 {
                // Ranged weapon — check the 7×5 visible window + weapon range.
                let dx = (pos.x as i32 - target_pos.x as i32).abs();
                let dy = (pos.y as i32 - target_pos.y as i32).abs();
                if dx > 7 || dy > 5 || cheb > weapon_range {
                    PlayerChaseOutcome::OutOfRange
                } else {
                    PlayerChaseOutcome::RangedStrike
                }
            } else if cheb > 1 {
                PlayerChaseOutcome::OutOfRange
            } else {
                PlayerChaseOutcome::Adjacent
            }
        }
    }

    /// Player `TDAttack` execute — `cract.cc:843-845` (`this->Attack()`) + the thrown-`RESULT`
    /// catch (`cract.cc:870-889`) specialized for players (`crplayer.cc:388-405`).
    ///
    /// Routes through [`Self::player_can_to_do_attack_chase`]. The melee **strike** is deferred
    /// (no player weapon-combat system yet); this drives the chase re-path on the attack beat and
    /// the `TARGETLOST` → `StopAttack` + `SendClearTarget` + `ToDoClear` + `SendResult` +
    /// `ToDoWait(1000)` + `ToDoStart` recovery (`crcombat.cc:456`, `crplayer.cc:393-402`).
    pub(crate) fn player_execute_attack(&mut self, cid: CreatureId) -> TodoExecuteKind {
        let conn_id = self.conn_for_creature(cid);
        match self.player_can_to_do_attack_chase(cid) {
            PlayerChaseOutcome::NoTarget => {
                // No attack target — idle drain complete, no re-arm (`crplayer.cc:388-405`).
                trace_creature_todo(self, cid, "player_attack_no_target");
                TodoExecuteKind::AttackDeferred
            }
            PlayerChaseOutcome::TargetLost => {
                // `StopAttack(0)` + `SendClearTarget` (`crcombat.cc:456`, `:513-522`).
                if let Some(conn) = conn_id {
                    self.player_stop_attack(conn, cid);
                } else if let Some(k) = self.creatures.get_mut(cid) {
                    let base = k.base_mut();
                    base.attack_target = None;
                    base.follow_target = None;
                }
                // `ToDoClear` + `SendResult(TARGETLOST)` + `ToDoWait(1000)` + `ToDoStart`
                // (`crplayer.cc:394-402`). `ToDoClear` drops the (already-popped) Attack.
                self.player_todo_clear(cid);
                if let Some(conn) = conn_id {
                    self.send_combat_result(conn, CombatResult::TargetLost);
                }
                self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
                trace_creature_todo(self, cid, "player_attack_target_lost");
                TodoExecuteKind::AttackDeferred
            }
            PlayerChaseOutcome::ChaseArmed => {
                // `ToDoGo` already enqueued at front by `player_can_to_do_attack_chase`; re-queue
                // `TDAttack` behind it so the chase drains then attack re-evaluates
                // (`cract.cc:1325` `ToDoGo` then `TDAttack`).
                let _ = self.enqueue_creature_attack(cid);
                trace_creature_todo(self, cid, "player_attack_chase_armed");
                TodoExecuteKind::AttackDeferred
            }
            PlayerChaseOutcome::Adjacent => {
                // `cheb ≤ 1` — strike range for any weapon. C++ `Attack()` checks `GetDistance()`
                // (`crcombat.cc:611-616`): range 1 → `CloseAttack`, range 2/3 → `DistanceAttack`/
                // `WandAttack` (a ranged weapon at cheb=1 is still within its range). The strike
                // bodies live in `player/combat/strike.rs` (melee) and `player/combat/ranged.rs`
                // (ranged); both handle `DelayAttack(200)` before + `DelayAttack(attackspeed)`
                // after, damage/defense/armor, `ActivateLearning`, and `StopAttack` on death.
                if let Some(target_id) = self
                    .creatures
                    .get(cid)
                    .and_then(|k| k.base().attack_target)
                {
                    let weapon_range = self.player_weapon_range(cid);
                    if weapon_range >= 2 {
                        self.player_ranged_attack_strike(cid, target_id);
                    } else {
                        self.player_close_attack_strike(cid, target_id);
                    }
                }
                // Re-arm `TDAttack` on the attack beat (post-strike `attackspeed` cadence is
                // already set by the strike; `todo_attack_delay_ms` reads `earliest_attack_ms`).
                let _ = self.enqueue_creature_attack(cid);
                let delay = self.todo_attack_delay_ms(cid).max(1);
                self.todo_start_from_action(cid, delay);
                trace_creature_todo(self, cid, "player_attack_strike");
                TodoExecuteKind::AttackDeferred
            }
            PlayerChaseOutcome::RangedStrike => {
                // Ranged weapon (bow/wand/throw) with target within weapon range and the 7×5
                // visible window — `crcombat.cc:617-638` dispatches to `DistanceAttack`/
                // `WandAttack`. The strike body lives in `player/combat/ranged.rs`; it handles
                // range/LoS checks, mana/ammo consumption, damage, `ActivateLearning`, and
                // `StopAttack` on target death.
                if let Some(target_id) = self
                    .creatures
                    .get(cid)
                    .and_then(|k| k.base().attack_target)
                {
                    self.player_ranged_attack_strike(cid, target_id);
                }
                let _ = self.enqueue_creature_attack(cid);
                let delay = self.todo_attack_delay_ms(cid).max(1);
                self.todo_start_from_action(cid, delay);
                trace_creature_todo(self, cid, "player_attack_ranged_strike");
                TodoExecuteKind::AttackDeferred
            }
            PlayerChaseOutcome::NoPath => {
                // Target reachable but no path found this beat — re-arm on the attack beat.
                // `DelayAttack(200)` matches the C++ pre-strike cadence (`crcombat.cc:608`).
                let _ = self.enqueue_creature_attack(cid);
                let server_ms = self.server_ms;
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().delay_attack_ms(server_ms, 200);
                }
                let delay = self.todo_attack_delay_ms(cid).max(1);
                self.todo_start_from_action(cid, delay);
                trace_creature_todo(self, cid, "player_attack_rearm");
                TodoExecuteKind::AttackDeferred
            }
            PlayerChaseOutcome::OutOfRange => {
                // `CHASE_MODE_NONE` + target not adjacent — C++ `Attack()` calls
                // `DelayAttack(200)` then `throw TARGETOUTOFRANGE` (`crcombat.cc:608,613-614`).
                // The `Execute` catch does `ToDoYield` (re-arm); no `SendResult` —
                // `TARGETOUTOFRANGE` falls through to `default: break` in `sending.cc:348`.
                let _ = self.enqueue_creature_attack(cid);
                let server_ms = self.server_ms;
                if let Some(k) = self.creatures.get_mut(cid) {
                    k.base_mut().delay_attack_ms(server_ms, 200);
                }
                let delay = self.todo_attack_delay_ms(cid).max(1);
                self.todo_start_from_action(cid, delay);
                trace_creature_todo(self, cid, "player_attack_out_of_range");
                TodoExecuteKind::AttackDeferred
            }
        }
    }

    /// Subset of `SetAttackDest` validation — `crcombat.cc:363-428`.
    ///
    /// Implements: target-NPC → `ATTACKNOTALLOWED` (`:404-407`); PZ (master or target) →
    /// `PROTECTIONZONE` (`:384-388`); `Distance > 8` → `TARGETLOST` (`:424-428`); invisible
    /// target (vs creature) → `TARGETLOST` (`:417-422`). Secure-mode / PVP `IsAttackJustified`
    /// (`:374-381`, `:468-474`) and `CheckRight(NO_ATTACK/ATTACK_EVERYWHERE)` are deferred to the
    /// player weapon-combat system.
    fn validate_player_attack_target(
        &self,
        cid: CreatureId,
        target_id: CreatureId,
    ) -> CombatResult {
        let (target_is_npc, target_pos) = match self.creatures.get(target_id) {
            Some(k) => (matches!(k, CreatureKind::Npc(_)), k.position()),
            None => return CombatResult::TargetLost,
        };
        if target_is_npc {
            return CombatResult::AttackNotAllowed;
        }
        let master_pos = self
            .creatures
            .get(cid)
            .map(|k| k.position())
            .unwrap_or(target_pos);
        let in_pz = self.tile_in_protection_zone(master_pos)
            || self.tile_in_protection_zone(target_pos);
        if in_pz {
            return CombatResult::ProtectionZone;
        }
        // Cross-floor / distance > 8 → TARGETLOST (`ObjectDistance` is INT_MAX across Z).
        if master_pos.z != target_pos.z || chebyshev(master_pos, target_pos) > 8 {
            return CombatResult::TargetLost;
        }
        // Invisible creature target — `crcombat.cc:417-422` (player vs non-player).
        if self.creatures.get(target_id).is_some_and(|k| {
            matches!(k, CreatureKind::Monster(_) | CreatureKind::Npc(_))
                && k.base().is_invisible()
        }) {
            return CombatResult::TargetLost;
        }
        CombatResult::NoError
    }

    pub(crate) fn tile_in_protection_zone(&self, pos: tfs_rust_common::Position) -> bool {
        self.map
            .get_tile(pos)
            .is_some_and(|t| t.body().zone == ZoneType::Protection)
    }

    /// `SendResult` — `sending.cc:285-357`: text via `SendMessage(TALK_FAILURE_MESSAGE, ...)`.
    fn send_combat_result(&mut self, conn_id: ConnId, result: CombatResult) {
        // 772 `sending.cc:348` `default: break` — `OUTOFAMMO` (38) has no message case, so no
        // text is sent. The catch path still does `ToDoWait(1000)` + `ToDoStart`
        // (`crplayer.cc:395-401`), which the caller handles.
        if matches!(result, CombatResult::OutOfAmmo) {
            return;
        }
        let msg = result.to_return_value().description();
        self.enqueue_outgoing(
            conn_id,
            send_text_message_simple(self.codec.failure_message_type(), msg).into_bytes(),
        );
    }

    /// Reverse wire-id → `CreatureId` lookup. Players use `guid`; monsters/NPCs use the low 32
    /// bits of the SlotMap key (`non_player_wire_id`).
    pub(crate) fn creature_by_wire_id(&self, wire_id: u32) -> Option<CreatureId> {
        if let Some(cid) = self.player_by_guid.get(&wire_id) {
            return Some(*cid);
        }
        self.creatures.iter().find_map(|(cid, k)| match k {
            CreatureKind::Monster(_) | CreatureKind::Npc(_) => {
                let native = (cid.data().as_ffi() & 0xFFFF_FFFF) as u32;
                (native == wire_id).then_some(cid)
            }
            _ => None,
        })
    }
}

/// Outcome of `CanToDoAttack` for the player `Attack` execute arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerChaseOutcome {
    /// No `attack_target` set — idle drain complete.
    NoTarget,
    /// Target gone / dead / out of range — caller should `StopAttack` + `SendResult(TARGETLOST)`.
    TargetLost,
    /// Chase `Go` armed — keep `Attack` deferred; re-arm on the `Go` wake.
    ChaseArmed,
    /// Target reachable but no path found this beat — re-arm on the attack beat.
    NoPath,
    /// `CHASE_MODE_NONE` + target not adjacent — C++ `Attack()` throws `TARGETOUTOFRANGE`
    /// (`crcombat.cc:611-614`). Re-arm without striking; `DelayAttack(200)` already applied.
    OutOfRange,
    /// Adjacent (cheb ≤ 1) — strike range. Strike deferred; re-arm on the attack beat.
    Adjacent,
    /// Ranged weapon (bow/wand/throw) with target within weapon range and LoS —
    /// `crcombat.cc:617-638` dispatches to `DistanceAttack`/`WandAttack`. The strike body lives
    /// in `player/combat/ranged.rs`; this outcome re-arms on the attack beat after the strike.
    RangedStrike,
}

/// Helper for the `SetAttackDest` early-out comparison without borrowing `self` mutably twice.
fn target_wire_id_to_creature(world: &GameWorld, wire_id: u32) -> Option<CreatureId> {
    world.creature_by_wire_id(wire_id)
}
