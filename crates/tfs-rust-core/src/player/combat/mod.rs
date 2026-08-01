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
//! unified ToDo engine. Melee / ranged / wand strikes live in `strike.rs` / `ranged.rs`.
//!
//! 772 has **no** separate `follow_target` semantics — follow == attack-with-`Following`
//! (`crcombat.cc:493-495` sets `ChaseMode = CHASE_MODE_CLOSE` when `Following`). `Attack()`
//! early-returns when `Following` (`crcombat.cc:532-534`) — chase only, never strike. We still set
//! `follow_target` so the shared `Go`/pathfinding arms (which key off `follow_target`) repath
//! toward the target on the attack beat.

pub(crate) mod fight_mode;
pub(crate) mod ranged;
pub(crate) mod skills;
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
use tfs_rust_common::WorldType;
use tfs_rust_net::outgoing_extra::send_text_message_simple;

use crate::creature::{ChaseMode, CreatureKind};
use crate::creature_todo::{trace_creature_todo, MONSTER_IDLE_WAIT_MS};
use crate::game_world::GameWorld;
use crate::idle_stimulus::TodoExecuteKind;
use crate::ids::CreatureId;
use crate::monster_ai::chebyshev;
use crate::player_flags::PLAYER_FLAG_IGNORE_PROTECTION_ZONE;
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
    /// `SECUREMODE` — secure mode blocks attacking an unmarked player (PVP). PC-4 wires the
    /// gate in `validate_player_attack_target` + `player_execute_attack` (PVP `IsAttackJustified`
    /// check, `crcombat.cc:374-381,563-568`). Skull/aggressor tracking is deferred.
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

        // Validate — `!Follow` gates (secure / PZ / profession / NoPvp / NPC) vs universal
        // distance + invisibility (`crcombat.cc:374-428`).
        let result = self.validate_player_attack_target(cid, target_id, follow);
        if result != CombatResult::NoError {
            self.player_stop_attack(conn_id, cid);
            // `CAttack` catch: `ToDoClear` + `SendResult` (unless NOERROR) + `ToDoYield`
            // (`receiving.cc:1149-1153`). `ToDoWait(1000)` is the idle-re-arm path (`crplayer.cc`).
            self.creature_todo_clear(cid);
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
            } else {
                // 772 `LatestAttackTime = 0` on `!Follow` only (`crcombat.cc:438`) — clears the
                // delayed-`StopAttack` flag, **not** `EarliestAttackTime`. Do **not** zero
                // `earliest_attack_ms` here or retargeting wipes attack-speed exhaust and allows
                // an immediate strike on the new target.
                base.latest_attack_round = 0;
            }
        }

        // PC-4 — `!Follow` path: `Target->AttackStimulus` + `Master->BlockLogout`
        // (`crcombat.cc:432-437`). Early-out above already skipped same dest+follow.
        if !follow {
            self.combat_on_attack_dest_changed(cid, target_id);
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

    /// 772 `TCombat::StopAttack(Delay)` — `crcombat.cc:513-522`.
    ///
    /// - `delay_rounds == 0`: clear `AttackDest` / follow; players get `SendClearTarget` (`0xA3`).
    /// - `delay_rounds > 0`: leave dest armed; set `LatestAttackTime = RoundNr + Delay`. The next
    ///   `Attack()` after that round expires the dest (`crcombat.cc:551-553`). Used by
    ///   `StartLogout(..., StopFight=false)` with delay **60** (`crmain.cc:414`,
    ///   `connections.cc:38` dead-connection logout).
    pub(crate) fn combat_stop_attack(&mut self, cid: CreatureId, delay_rounds: u32) {
        self.combat_stop_attack_with_conn(None, cid, delay_rounds);
    }

    fn combat_stop_attack_with_conn(
        &mut self,
        conn_hint: Option<ConnId>,
        cid: CreatureId,
        delay_rounds: u32,
    ) {
        if delay_rounds == 0 {
            let was_attacking = self
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().attack_target.is_some());
            let is_player = self
                .creatures
                .get(cid)
                .is_some_and(|k| matches!(k, CreatureKind::Player(_)));
            if let Some(k) = self.creatures.get_mut(cid) {
                let base = k.base_mut();
                base.attack_target = None;
                base.follow_target = None;
            }
            if was_attacking && is_player {
                let conn_id = conn_hint.or_else(|| self.conn_for_creature(cid));
                if let Some(conn_id) = conn_id {
                    self.enqueue_encoded(conn_id, self.codec.encode_clear_target());
                }
                trace_creature_todo(self, cid, "combat_stop_attack");
            }
        } else if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().latest_attack_round = self.round_nr.saturating_add(delay_rounds);
        }
    }

    /// 772 `TCombat::StopAttack(0)` — `crcombat.cc:513-522`: clear `AttackDest` and, for players,
    /// `SendClearTarget` (`0xA3`).
    pub(crate) fn player_stop_attack(&mut self, conn_id: ConnId, cid: CreatureId) {
        self.combat_stop_attack_with_conn(Some(conn_id), cid, 0);
    }

    /// 772 `StartLogout(Force, StopFight)` combat half — `crmain.cc:414`.
    ///
    /// `stop_fight` true → immediate `StopAttack(0)`; false → `StopAttack(60)` (RoundNr).
    pub(crate) fn creature_start_logout_stop_fight(&mut self, cid: CreatureId, stop_fight: bool) {
        const DELAYED_STOP_ATTACK_ROUNDS: u32 = 60;
        self.combat_stop_attack(
            cid,
            if stop_fight {
                0
            } else {
                DELAYED_STOP_ATTACK_ROUNDS
            },
        );
    }

    /// `Attack()` delayed-stop expire — `crcombat.cc:551-553`.
    ///
    /// Returns `true` when the dest was cleared (caller must not strike). Silent — no
    /// `TARGETLOST` / `SendResult`. Skipped while `Following` (Attack early-returns).
    pub(crate) fn combat_expire_delayed_stop_attack(&mut self, cid: CreatureId) -> bool {
        let (following, latest) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => {
                // Player `Following` == `follow_target.is_some()` (`SetAttackDest` follow arm).
                (p.base.follow_target.is_some(), p.base.latest_attack_round)
            }
            Some(k) => {
                // Monster chase `follow_target` is **not** `Combat.Following` — Attack() still
                // runs expire for monsters (`crcombat.cc:532` Following is player follow-mode).
                (false, k.base().latest_attack_round)
            }
            None => return false,
        };
        if following || latest == 0 || latest >= self.round_nr {
            return false;
        }
        self.combat_stop_attack(cid, 0);
        true
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
        let effective_chase = if following {
            ChaseMode::Close
        } else {
            chase_mode
        };

        if effective_chase == ChaseMode::Close {
            if cheb > 1 {
                // `ToDoGo(target, false, 3)` — close chase (`crcombat.cc:498-499`). Reuse the generic
                // player-aware pathfinder (`get_creature_path_to` dispatches to
                // `tile_query_add_player` for the moving player). `MaxSteps=3` limits the
                // walk to 3 steps before combat re-evaluates target position.
                let Some(steps) = self.get_creature_path_to(cid, target_pos, 1, 1, 3) else {
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
    /// Routes through [`Self::player_can_to_do_attack_chase`]. When `Following`, `Attack()`
    /// early-returns after chase (`crcombat.cc:532-534`) — never strikes. Otherwise melee /
    /// ranged / wand strikes run from `strike.rs` / `ranged.rs`.
    pub(crate) fn player_execute_attack(&mut self, cid: CreatureId) -> TodoExecuteKind {
        // `Attack()` early: `AttackDest == 0 || Following` → return (`crcombat.cc:532-534`).
        // Delayed `StopAttack` expire runs only on the non-follow arm (`:551-553`).
        if self.combat_expire_delayed_stop_attack(cid) {
            trace_creature_todo(self, cid, "player_attack_delayed_stop_expired");
            return TodoExecuteKind::AttackDeferred;
        }

        let following = self.creatures.get(cid).is_some_and(|k| {
            matches!(k, CreatureKind::Player(_)) && k.base().follow_target.is_some()
        });

        let conn_id = self.conn_for_creature(cid);
        // `ToDoAttack` → `CanToDoAttack` first (`cract.cc:1354`); chase still runs while Following.
        let outcome = self.player_can_to_do_attack_chase(cid);

        // PC-4 — `Attack()` re-validation for non-lost targets (`crcombat.cc:563-606`):
        // secure-mode / profession NONE / `NO_ATTACK` / NoPvp peaceful / PZ + `BlockLogout(60)`.
        // Skipped entirely when `Following` (`Attack` returns before these checks).
        if !following
            && !matches!(
                outcome,
                PlayerChaseOutcome::NoTarget | PlayerChaseOutcome::TargetLost
            )
        {
            if let Some(target_id) =
                self.creatures.get(cid).and_then(|k| k.base().attack_target)
            {
                // Secure-mode PVP gate — `crcombat.cc:563-568`.
                if self.player_secure_mode_blocks_attack(cid, target_id) {
                    return self.player_attack_abort_with_result(
                        cid,
                        conn_id,
                        CombatResult::SecureMode,
                        "player_attack_secure_mode_blocked",
                    );
                }

                // Profession NONE / `!allowPvp` vs player — `crcombat.cc:580-586`.
                if self.player_vocation_blocks_pvp_attack(cid, target_id) {
                    return self.player_attack_abort_with_result(
                        cid,
                        conn_id,
                        CombatResult::AttackNotAllowed,
                        "player_attack_vocation_pvp_blocked",
                    );
                }

                // `CheckRight(NO_ATTACK)` → `ATTACKNOTALLOWED` (`crcombat.cc:589-593`).
                if self.player_attack_blocked_by_right(cid) {
                    return self.player_attack_abort_with_result(
                        cid,
                        conn_id,
                        CombatResult::AttackNotAllowed,
                        "player_attack_right_blocked",
                    );
                }

                // NON_PVP peaceful×peaceful — `CanToDoAttack` (`crcombat.cc:476-483`).
                if self.player_nopvp_peaceful_blocks_attack(cid, target_id) {
                    return self.player_attack_abort_with_result(
                        cid,
                        conn_id,
                        CombatResult::AttackNotAllowed,
                        "player_attack_nopvp_peaceful_blocked",
                    );
                }

                // PZ on attacker or target — `Attack()` always (`crcombat.cc:595-599`).
                let (master_pos, target_pos) = match (
                    self.creatures.get(cid).map(|k| k.position()),
                    self.creatures.get(target_id).map(|k| k.position()),
                ) {
                    (Some(a), Some(b)) => (a, b),
                    _ => {
                        return self.player_attack_abort_with_result(
                            cid,
                            conn_id,
                            CombatResult::TargetLost,
                            "player_attack_target_lost",
                        );
                    }
                };
                if self.tile_in_protection_zone(master_pos)
                    || self.tile_in_protection_zone(target_pos)
                {
                    return self.player_attack_abort_with_result(
                        cid,
                        conn_id,
                        CombatResult::ProtectionZone,
                        "player_attack_protection_zone",
                    );
                }

                // `BlockLogout(60)` on attacker + target — `crcombat.cc:601-602`.
                let target_is_player = self
                    .creatures
                    .get(target_id)
                    .is_some_and(|k| matches!(k, CreatureKind::Player(_)));
                self.player_block_logout_infight(cid, target_is_player);
                self.player_block_logout_infight(target_id, false);
            }
        }

        match outcome {
            PlayerChaseOutcome::NoTarget => {
                trace_creature_todo(self, cid, "player_attack_no_target");
                TodoExecuteKind::AttackDeferred
            }
            PlayerChaseOutcome::TargetLost => {
                if let Some(conn) = conn_id {
                    self.player_stop_attack(conn, cid);
                } else if let Some(k) = self.creatures.get_mut(cid) {
                    let base = k.base_mut();
                    base.attack_target = None;
                    base.follow_target = None;
                }
                self.creature_todo_clear(cid);
                if let Some(conn) = conn_id {
                    self.send_combat_result(conn, CombatResult::TargetLost);
                }
                self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
                trace_creature_todo(self, cid, "player_attack_target_lost");
                TodoExecuteKind::AttackDeferred
            }
            PlayerChaseOutcome::ChaseArmed => {
                let _ = self.enqueue_creature_attack(cid);
                trace_creature_todo(self, cid, "player_attack_chase_armed");
                TodoExecuteKind::AttackDeferred
            }
            PlayerChaseOutcome::Adjacent | PlayerChaseOutcome::RangedStrike if following => {
                // `Attack()` returns when Following — chase already handled; re-arm without strike.
                let _ = self.enqueue_creature_attack(cid);
                let delay = self.todo_attack_delay_ms(cid).max(1);
                self.todo_start_from_action(cid, delay);
                trace_creature_todo(self, cid, "player_attack_following");
                TodoExecuteKind::AttackDeferred
            }
            PlayerChaseOutcome::Adjacent => {
                if let Some(target_id) =
                    self.creatures.get(cid).and_then(|k| k.base().attack_target)
                {
                    let weapon_range = self.player_weapon_range(cid);
                    if weapon_range >= 2 {
                        self.player_ranged_attack_strike(cid, target_id);
                    } else {
                        self.player_close_attack_strike(cid, target_id);
                    }
                }
                let _ = self.enqueue_creature_attack(cid);
                let delay = self.todo_attack_delay_ms(cid).max(1);
                self.todo_start_from_action(cid, delay);
                trace_creature_todo(self, cid, "player_attack_strike");
                TodoExecuteKind::AttackDeferred
            }
            PlayerChaseOutcome::RangedStrike => {
                if let Some(target_id) =
                    self.creatures.get(cid).and_then(|k| k.base().attack_target)
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

    /// `StopAttack` + `SendResult` + `ToDoWait(1000)` + `ToDoStart` — Attack / CanToDoAttack throw path.
    fn player_attack_abort_with_result(
        &mut self,
        cid: CreatureId,
        conn_id: Option<ConnId>,
        result: CombatResult,
        trace: &str,
    ) -> TodoExecuteKind {
        if let Some(conn) = conn_id {
            self.player_stop_attack(conn, cid);
        } else if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            base.attack_target = None;
            base.follow_target = None;
        }
        self.creature_todo_clear(cid);
        if let Some(conn) = conn_id {
            self.send_combat_result(conn, result);
        }
        self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
        trace_creature_todo(self, cid, trace);
        TodoExecuteKind::AttackDeferred
    }

    /// Subset of `SetAttackDest` validation — `crcombat.cc:363-428`.
    ///
    /// `follow == false` (`!Follow`): secure-mode, `NO_ATTACK`, profession/`allowPvp`, NPC, PZ
    /// (unless `IgnoreProtectionZone` / `ATTACK_EVERYWHERE`), NON_PVP peaceful×peaceful.
    /// Always: distance > 8 / cross-floor → `TARGETLOST`; invisible non-player → `TARGETLOST`.
    /// Skull / `RecordAttack` remain deferred (`IsAttackJustified` stub).
    fn validate_player_attack_target(
        &self,
        cid: CreatureId,
        target_id: CreatureId,
        follow: bool,
    ) -> CombatResult {
        let (target_is_npc, target_pos, target_is_player) = match self.creatures.get(target_id) {
            Some(k) => (
                matches!(k, CreatureKind::Npc(_)),
                k.position(),
                matches!(k, CreatureKind::Player(_)),
            ),
            None => return CombatResult::TargetLost,
        };

        if !follow {
            // Secure-mode PVP gate — `crcombat.cc:374-381`.
            if target_is_player && self.player_secure_mode_blocks_attack(cid, target_id) {
                return CombatResult::SecureMode;
            }

            // `CheckRight(NO_ATTACK)` → `ATTACKNOTALLOWED` (`crcombat.cc:391-394`).
            if self.player_attack_blocked_by_right(cid) {
                return CombatResult::AttackNotAllowed;
            }

            // Profession NONE / `!allowPvp` vs player — `crcombat.cc:396-401`.
            if self.player_vocation_blocks_pvp_attack(cid, target_id) {
                return CombatResult::AttackNotAllowed;
            }

            if target_is_npc {
                return CombatResult::AttackNotAllowed;
            }

            let master_pos = self
                .creatures
                .get(cid)
                .map(|k| k.position())
                .unwrap_or(target_pos);
            // PZ — skipped when `ATTACK_EVERYWHERE` / `IgnoreProtectionZone` (`crcombat.cc:383-388`).
            if !self.player_has_flag(cid, PLAYER_FLAG_IGNORE_PROTECTION_ZONE) {
                let in_pz = self.tile_in_protection_zone(master_pos)
                    || self.tile_in_protection_zone(target_pos);
                if in_pz {
                    return CombatResult::ProtectionZone;
                }
            }

            // NON_PVP + both peaceful — `crcombat.cc:409-414`.
            if self.player_nopvp_peaceful_blocks_attack(cid, target_id) {
                return CombatResult::AttackNotAllowed;
            }
        }

        let master_pos = self
            .creatures
            .get(cid)
            .map(|k| k.position())
            .unwrap_or(target_pos);
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

    /// 772 `TCreature::IsPeaceful` / `TMonster::IsPeaceful` — `crmain.cc:900`, `crnonpl.cc:2295`.
    ///
    /// Players (and NPCs) are peaceful. Monsters are peaceful only when their master is a player
    /// (player summons).
    pub(crate) fn creature_is_peaceful(&self, cid: CreatureId) -> bool {
        match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => m.base.master.is_some_and(|mid| {
                matches!(self.creatures.get(mid), Some(CreatureKind::Player(_)))
            }),
            Some(CreatureKind::Player(_)) | Some(CreatureKind::Npc(_)) => true,
            None => true,
        }
    }

    /// `allowPvp == false` (772 `PROFESSION_NONE`) attacker vs player, unless
    /// `IgnoreProtectionZone` / `ATTACK_EVERYWHERE` (`crcombat.cc:396-401,580-586`).
    pub(crate) fn player_vocation_blocks_pvp_attack(
        &self,
        attacker: CreatureId,
        target: CreatureId,
    ) -> bool {
        let Some(CreatureKind::Player(a)) = self.creatures.get(attacker) else {
            return false;
        };
        if a.vocation_profile.allow_pvp {
            return false;
        }
        if !matches!(self.creatures.get(target), Some(CreatureKind::Player(_))) {
            return false;
        }
        !self.player_has_flag(attacker, PLAYER_FLAG_IGNORE_PROTECTION_ZONE)
    }

    /// `WorldType == NON_PVP` && both peaceful, unless attacker has `ATTACK_EVERYWHERE`
    /// (`crcombat.cc:409-414,476-483`).
    pub(crate) fn player_nopvp_peaceful_blocks_attack(
        &self,
        attacker: CreatureId,
        target: CreatureId,
    ) -> bool {
        if self.pvp_config.world_type != WorldType::NoPvp {
            return false;
        }
        if !self.creature_is_peaceful(attacker) || !self.creature_is_peaceful(target) {
            return false;
        }
        // Non-players as master still blocked; players may bypass with ATTACK_EVERYWHERE.
        if !matches!(self.creatures.get(attacker), Some(CreatureKind::Player(_))) {
            return true;
        }
        !self.player_has_flag(attacker, PLAYER_FLAG_IGNORE_PROTECTION_ZONE)
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

    /// Reverse wire-id → `CreatureId` lookup. Players use `guid`; monsters/NPCs use
    /// the auto-incrementing `wire_id` assigned at spawn (C++ `Monster::setID`).
    pub(crate) fn creature_by_wire_id(&self, wire_id: u32) -> Option<CreatureId> {
        if let Some(cid) = self.player_by_guid.get(&wire_id) {
            return Some(*cid);
        }
        self.creatures.iter().find_map(|(cid, k)| match k {
            CreatureKind::Monster(m) => {
                let native = if m.wire_id != 0 { m.wire_id } else { (cid.data().as_ffi() & 0xFFFF_FFFF) as u32 };
                (native == wire_id).then_some(cid)
            }
            CreatureKind::Npc(n) => {
                let native = if n.wire_id != 0 { n.wire_id } else { (cid.data().as_ffi() & 0xFFFF_FFFF) as u32 };
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

#[cfg(test)]
mod set_attack_dest_tests {
    use tfs_rust_common::Position;

    use crate::login_out::creature_wire_id;
    use crate::sim_harness::{
        ensure_walkable_tile, insert_monster, insert_player, test_player, TEST_SYNTHETIC_GROUND_WP,
    };

    use super::*;

    #[test]
    fn set_attack_dest_retarget_preserves_attack_exhaust() {
        // 772 clears `LatestAttackTime`, not `EarliestAttackTime` (`crcombat.cc:438`).
        let mut world = crate::sim_harness::beat_driven_test_world();
        let ppos = Position::new(100, 100, 7);
        let m1 = Position::new(101, 100, 7);
        let m2 = Position::new(100, 101, 7);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, m1, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, m2, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        let a = insert_monster(&mut world, "Rat", m1, 100);
        let b = insert_monster(&mut world, "Rat", m2, 100);
        world.map.register_creature_at(ppos, player);
        world.map.register_creature_at(m1, a);
        world.map.register_creature_at(m2, b);

        world.server_ms = 5_000;
        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().earliest_attack_ms = 7_000; // mid-exhaust after a prior strike
        }

        let wire_b = creature_wire_id(b, world.creatures.get(b).unwrap());
        let conn = tfs_rust_common::ConnId(1);
        world.register_conn_mapping(conn, player);
        let result = world.player_set_attack_dest(conn, player, wire_b, false);
        assert_eq!(result, CombatResult::NoError);
        let earliest = world.creatures.get(player).unwrap().base().earliest_attack_ms;
        assert_eq!(
            earliest, 7_000,
            "retarget must not clear EarliestAttackTime / attack-speed exhaust"
        );
        let delay = world.todo_attack_delay_ms(player);
        assert_eq!(delay, 2_000, "ToDoStart must wait remaining exhaust before next strike");
    }

    #[test]
    fn delayed_stop_attack_schedules_latest_attack_round() {
        // `StopAttack(60)` → `LatestAttackTime = RoundNr + 60` (`crcombat.cc:520`).
        let mut world = crate::sim_harness::beat_driven_test_world();
        let ppos = Position::new(100, 100, 7);
        let mpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        let mon = insert_monster(&mut world, "Rat", mpos, 100);
        world.map.register_creature_at(ppos, player);
        world.map.register_creature_at(mpos, mon);
        world.round_nr = 100;
        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().attack_target = Some(mon);
        }

        world.creature_start_logout_stop_fight(player, false);
        let base = world.creatures.get(player).unwrap().base();
        assert_eq!(base.attack_target, Some(mon), "delayed stop must keep AttackDest");
        assert_eq!(base.latest_attack_round, 160);
    }

    #[test]
    fn attack_expires_delayed_stop_when_round_passes() {
        // `LatestAttackTime != 0 && LatestAttackTime < RoundNr` → `StopAttack(0)` (`crcombat.cc:551`).
        let mut world = crate::sim_harness::beat_driven_test_world();
        let ppos = Position::new(100, 100, 7);
        let mpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        let mon = insert_monster(&mut world, "Rat", mpos, 100);
        world.map.register_creature_at(ppos, player);
        world.map.register_creature_at(mpos, mon);
        let conn = tfs_rust_common::ConnId(1);
        world.register_conn_mapping(conn, player);

        world.round_nr = 50;
        if let Some(k) = world.creatures.get_mut(player) {
            let b = k.base_mut();
            b.attack_target = Some(mon);
            b.latest_attack_round = 40; // already expired
        }

        assert!(world.combat_expire_delayed_stop_attack(player));
        let base = world.creatures.get(player).unwrap().base();
        assert!(base.attack_target.is_none());
        // C++ StopAttack(0) does not clear LatestAttackTime; leave as-is.
    }

    #[test]
    fn delayed_stop_not_expired_at_exact_deadline_round() {
        // Condition is `LatestAttackTime < RoundNr`, not `<=` (`crcombat.cc:551`).
        let mut world = crate::sim_harness::beat_driven_test_world();
        let ppos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        world.round_nr = 40;
        if let Some(k) = world.creatures.get_mut(player) {
            let b = k.base_mut();
            b.attack_target = Some(player); // any Some
            b.latest_attack_round = 40;
        }
        assert!(!world.combat_expire_delayed_stop_attack(player));
        assert!(world
            .creatures
            .get(player)
            .unwrap()
            .base()
            .attack_target
            .is_some());
    }

    #[test]
    fn set_attack_dest_clears_latest_attack_round() {
        let mut world = crate::sim_harness::beat_driven_test_world();
        let ppos = Position::new(100, 100, 7);
        let mpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        let mon = insert_monster(&mut world, "Rat", mpos, 100);
        world.map.register_creature_at(ppos, player);
        world.map.register_creature_at(mpos, mon);
        let conn = tfs_rust_common::ConnId(1);
        world.register_conn_mapping(conn, player);

        if let Some(k) = world.creatures.get_mut(player) {
            k.base_mut().latest_attack_round = 999;
        }
        let wire = creature_wire_id(mon, world.creatures.get(mon).unwrap());
        assert_eq!(
            world.player_set_attack_dest(conn, player, wire, false),
            CombatResult::NoError
        );
        assert_eq!(
            world.creatures.get(player).unwrap().base().latest_attack_round,
            0,
            "SetAttackDest !Follow must clear LatestAttackTime"
        );
    }

    #[test]
    fn delayed_stop_skipped_while_following() {
        let mut world = crate::sim_harness::beat_driven_test_world();
        let ppos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        world.round_nr = 100;
        if let Some(k) = world.creatures.get_mut(player) {
            let b = k.base_mut();
            b.attack_target = Some(player);
            b.follow_target = Some(player);
            b.latest_attack_round = 1;
        }
        assert!(
            !world.combat_expire_delayed_stop_attack(player),
            "Attack() returns early when Following — no expire"
        );
        assert!(world
            .creatures
            .get(player)
            .unwrap()
            .base()
            .attack_target
            .is_some());
    }

    #[test]
    fn following_adjacent_does_not_strike() {
        // `Attack()` early-returns when Following (`crcombat.cc:532-534`) — chase only.
        let mut world = crate::sim_harness::beat_driven_test_world();
        let ppos = Position::new(100, 100, 7);
        let mpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, ppos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Hero", ppos));
        let mon = insert_monster(&mut world, "Rat", mpos, 100);
        world.map.register_creature_at(ppos, player);
        world.map.register_creature_at(mpos, mon);
        if let Some(k) = world.creatures.get_mut(player) {
            let b = k.base_mut();
            b.attack_target = Some(mon);
            b.follow_target = Some(mon);
            b.chase_mode = ChaseMode::Close;
        }
        let hp_before = world.creatures.get(mon).unwrap().base().health;
        let _ = world.player_execute_attack(player);
        let hp_after = world.creatures.get(mon).unwrap().base().health;
        assert_eq!(hp_before, hp_after, "Following must not deal weapon damage");
    }

    #[test]
    fn vocation_without_allow_pvp_cannot_attack_player() {
        // `PROFESSION_NONE` / `!allowPvp` → ATTACKNOTALLOWED (`crcombat.cc:396-401`).
        let mut world = crate::sim_harness::beat_driven_test_world();
        let apos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, apos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, bpos, TEST_SYNTHETIC_GROUND_WP);
        let a = insert_player(&mut world, test_player("Rookie", apos));
        let mut bob = test_player("Bob", bpos);
        bob.guid = 2;
        let b = insert_player(&mut world, bob);
        world.map.register_creature_at(apos, a);
        world.map.register_creature_at(bpos, b);
        assert!(matches!(
            world.creatures.get(a),
            Some(CreatureKind::Player(p)) if !p.vocation_profile.allow_pvp
        ));
        assert_eq!(
            world.validate_player_attack_target(a, b, false),
            CombatResult::AttackNotAllowed
        );
        // Follow skips the !Follow vocation gate.
        assert_eq!(
            world.validate_player_attack_target(a, b, true),
            CombatResult::NoError
        );
    }

    #[test]
    fn nopvp_peaceful_blocks_player_vs_player() {
        // NON_PVP + both peaceful → ATTACKNOTALLOWED (`crcombat.cc:409-414`).
        let mut world = crate::sim_harness::beat_driven_test_world();
        world.pvp_config.world_type = tfs_rust_common::WorldType::NoPvp;
        let apos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, apos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, bpos, TEST_SYNTHETIC_GROUND_WP);
        let mut alice = test_player("Alice", apos);
        alice.vocation_profile.allow_pvp = true;
        alice.guid = 1;
        let mut bob = test_player("Bob", bpos);
        bob.vocation_profile.allow_pvp = true;
        bob.guid = 2;
        let a = insert_player(&mut world, alice);
        let b = insert_player(&mut world, bob);
        world.map.register_creature_at(apos, a);
        world.map.register_creature_at(bpos, b);
        assert_eq!(
            world.validate_player_attack_target(a, b, false),
            CombatResult::AttackNotAllowed
        );
        // Wild monsters are not peaceful — still attackable in NoPvp.
        let mpos = Position::new(100, 101, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        let mon = insert_monster(&mut world, "Rat", mpos, 100);
        world.map.register_creature_at(mpos, mon);
        assert_eq!(
            world.validate_player_attack_target(a, mon, false),
            CombatResult::NoError
        );
    }
}
