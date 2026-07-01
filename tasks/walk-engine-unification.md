# Walk Engine Unification — Migration Plan

**Date:** 2026-07-01 (updated 2026-07-01)
**Goal:** One walk/chase engine for both eras, based on the **CipSoft 772 decompile** ToDo model
(`TCreature::Execute` → `Go`/`Attack` → `IdleStimulus` → `Combat.CanToDoAttack`). Retire the
TFS-style parallel paths (`listWalkDir`/`eventWalk` deadlines, the 772-player `walk_queue` split,
`goToFollowCreature`/`onThink`-poll/`onCreatureMove` follow machinery).
**Decision + rationale:** see `tasks/player-walk-audit.md` → "ARCHITECTURE DECISION".
**772 spec source:** `reference/cipsoft-772/tibia-game-master/src/` — `cract.cc`, `crmain.cc`,
`crplayer.cc`, `crcombat.cc`, `receiving.cc`, `sending.cc`, `operate.cc`. **(Not** tvp-772
`gameserver/src/` — that is 772 *wire* only. **Not** the `chase_*` harness files.)

> **Work order:** get **772 parity first** (Phase 0 → Phase 1). 1098 (Phase 2) is deferred and
> deliberately under-specified until the 772 path is proven.

---

## Status

| Phase | Status | Notes |
|-------|--------|-------|
| **Phase 0** | ✅ DONE | `ChaseMode` moved to `CreatureBase`; `creature_uses_todo_execute` widened to players; clock seam documented. |
| **Phase 1.1** | ✅ DONE | `player_move_request` / `player_auto_walk_path` / `player_stop_auto_walk` route through ToDo on 772; `clear_todo_772` deleted. |
| **Phase 1.2** | ✅ DONE | `player_idle_stimulus` arm added; `request_idle_stimulus` widened to players. Player `Attack` execute defers to 1.4. |
| **Phase 1.3** | ✅ DONE | `on_walk` blocked-step handling widened to all ToDo creatures; unconditional `ToDoClear` + `IdleStimulus`; player-only `force_update_follow_path` branch removed. |
| **Phase 1.4** | ⬜ DEFERRED | Attack/follow packets → `SetAttackDest` + `CanToDoAttack` chase. |
| **Phase 1.5** | ⬜ DEFERRED | Drunk stagger (CipSoft formula + `ToDoClear` + `ToDoTalk`). |
| **Phase 1.6** | ⬜ DEFERRED | Reference/comment cleanup (N2, N3, N4). |
| **Phase 1.7** | ⬜ DEFERRED | Player ToDo/idle tests. |
| **Phase 2** | ⬜ DEFERRED | 1098 unification. |

**Verification (2026-07-01):** `cargo check` 0 errors · `cargo clippy` 0 errors (55 pre-existing warnings) · `cargo test` 457 passed, 2 ignored, 0 failed.

**Additional change (user-requested):** `MonsterChaseMode` renamed to `ChaseMode` and moved from
`Monster` to `CreatureBase` (shared by players + monsters on the unified chase path). `Monster`
keeps `last_combat_trace` (harness-only dedupe) keyed on `ChaseMode` from base.

---

## What unifies vs what stays era-specific

| Concern | Unify (one code path) | Era-specific (where) |
|---------|-----------------------|----------------------|
| Scheduling | `Execute` ToDo queue + `IdleStimulus` drain — players **and** monsters | — |
| Failed step | `ToDoClear` + `ToDoYield` → `IdleStimulus` | — |
| Follow / chase | `SetAttackDest(id, Follow)` + `CanToDoAttack` (`ChaseMode`) | — |
| Step-duration formula | shared `get_step_duration` seam | 772 linear `Waypoints*1000/GetSpeed` ceil→Beat vs 1098 log `A·ln+C` → `MechanicsProfile` |
| Effective speed | shared | `2·go+80` (772) vs clamped log (1098) → `MechanicsProfile` |
| Scheduler clock | shared `ToDoQueue` API | Beat/`server_ms` (772) vs `Instant` (1098) → clock adapter |
| Wire bytes | — | `0x6B`/`0xB5`/`0x6D` stay in `ProtocolCodec` |

---

## Phase 0 — Clock-agnostic ToDo scheduler *(prerequisite for 772-player work, small)*

Today the ToDo engine is hard-wired to the 772 beat clock (`~18` `server_ms`/`next_wakeup`/Beat
references in `creature_todo.rs`, plus `todo_queue.rs`, `idle_stimulus.rs`). For 772-player parity
this can stay Beat-based — **but** confirm the engine has no *monster-only* assumptions before
routing players through it.

- [x] Audit `creature_todo.rs` / `todo_queue.rs` / `idle_stimulus.rs` for `CreatureKind::Monster`
      gates that must widen to players (e.g. `creature_uses_todo_execute`, the `idle_stimulus`
      `match … _ => {}` arm, `request_idle_stimulus` monster guard).
      — **Done:** `creature_uses_todo_execute` widened to `Player`; `idle_stimulus` player arm
      added (1.2); `request_idle_stimulus` widened (1.2). `MonsterChaseMode` → `ChaseMode` moved
      to `CreatureBase`.
- [x] Decide the clock seam now (even if 1098 lands later): a `now_beat()` / `schedule_at()` API
      that is Beat-quantized on 772 and pass-through `Instant` on 1098. Document it; do **not**
      implement the 1098 side yet.
      — **Done:** clock seam documented in `creature_todo.rs` module header. 1098 side not
      implemented (deferred to Phase 2).

**Acceptance:** you can enumerate every monster-only branch that Phase 1 must widen, and the
scheduler API no longer *conceptually* assumes "monster".

---

## Phase 1 — 772 PLAYER PARITY *(primary focus)*

Route 772 players through the same ToDo/`Execute`/`IdleStimulus`/`Combat` path monsters already
use. This closes audit bugs **P1, P2, P3, P4, P6** and finding **N1**.

### 1.1 Player walk requests → ToDo (`CGoDirection` / `CGoPath`)
Decompile: `receiving.cc:120-199`, `cract.cc` `ToDoGo`/`ToDoAdd`/`ToDoStart` (`:1050-1107`,
`:991-1024`).
- [x] `player_move_request` (`walk/mod.rs`): replaced `clear_todo_772` + `walk_queue` +
      `add_event_walk` with `ToDoClear`(→snapback if pending Go) → `TDGo` entry → `ToDoStart`
      on 772. 1098 path unchanged.
- [x] `player_auto_walk_path` (`walk/mod.rs`): same — enqueues directions into `walk_queue` +
      one `Go` action + `ToDoStart` on 772. 1098 path unchanged.
- [x] `player_stop_auto_walk` → `ToDoStop` (`receiving.cc:201-211`, `cract.cc:1002-1008`).
      772 clears ToDo queue + walk queue + cancels wakeup. 1098 keeps `cancel_next_walk`.
- [x] Delete `clear_todo_772` once no longer referenced.
      — **Done:** replaced by `player_todo_clear` / `player_todo_clear_with_snapback`.

**Acceptance:** a 772 player single-step and autowalk both execute via `Execute`; a new move mid-walk
issues `ToDoClear`+snapback then restarts (matches `CGoDirection`).

### 1.2 `TPlayer::IdleStimulus` arm
Decompile: `crplayer.cc:388-405`.
- [x] Add a player arm to `idle_stimulus` (`idle_stimulus.rs`): `player_idle_stimulus` handles
      **only** `Combat.AttackDest` — `ToDoAttack` → `ToDoStart` if `attack_target` is set; else
      goes idle cleanly (no re-arm). The thrown `RESULT` → `ToDoClear` + `SendResult` +
      `ToDoWait(1000)` path is deferred to 1.4 (player Attack execute).
- [x] Widen `request_idle_stimulus` (`idle_stimulus.rs`) to players — now uses
      `creature_uses_todo_execute` (which includes players) instead of monster-only guard.
- [x] There is **no** separate follow re-path in player idle — follow lives in Combat (1.4).
      — **Confirmed:** `player_idle_stimulus` does no follow re-path.

**Acceptance:** a 772 player with no attack target goes idle cleanly after a walk drains (no
re-arm); with an attack target, idle re-issues the attack/chase.

### 1.3 Failed-step handling on the shared path (closes P1 + P4)
Decompile: `Execute` catch `cract.cc:870-889`; `ToDoClear` `:953-989`; `SendResult` order
`sending.cc:285-357`.
- [x] On a blocked player step, the shared `Execute` catch already does the right thing for
      monsters — players now hit it too: `ToDoClear()` (clears the **whole** queue) →
      `ToDoYield()` → `IdleStimulus`. Removed the player-specific `on_walk` error branch
      that only set `force_update_follow_path`. Widened guard from `CreatureKind::Monster` to
      `creature_uses_todo_execute` (includes players). `ToDoClear` is now unconditional
      (clears whole queue regardless of attack state, per `cract.cc:871`).
- [x] Wire order: text (`SendResult`) **then** `0xB5` snapback — already correct; kept.
      Snapback for `MOVENOTPOSSIBLE` is emitted by `SendResult`.

**Acceptance:** 772 player auto-walking into a wall stops cleanly (queue emptied, one text + one
snapback), no per-remaining-step stutter.

### 1.4 Attack + follow packets → unified `SetAttackDest` (closes P3, enables P2)
Decompile: `receiving.cc:1133-1155` (`CAttack(Follow)`), `crcombat.cc:357-511`
(`SetAttackDest` / `CanToDoAttack`), `StopAttack` `:513-522`.
- [ ] Handle `GamePacket::Attack` and `GamePacket::Follow` in `game_loop.rs` (currently fall to
      the `_ =>` catch-all, `:418`) via **one** call: `SetAttackDest(target, follow)` →
      `ToDoAttack` → `ToDoStart`. Attack = `follow=false`, Follow = `follow=true`.
- [ ] Handle `GamePacket::CancelAttackAndFollow` → `SetAttackDest(0)` (→ `StopAttack` →
      `SendClearTarget`) and `ToDoStop`.
- [ ] Implement/verify `CanToDoAttack` chase: `Following ⇒ ChaseMode=CLOSE`;
      `CLOSE & Distance>1 ⇒ ToDoGo(target, false, 3)`; `RANGE` keeps distance 4
      (may already exist for monsters — reuse).
- [ ] Note: 772 has **no** separate `follow_target`; follow == attack-with-`Following`. Do not add
      TFS `goToFollowCreature`.

**Acceptance:** a 772 player can attack and follow a creature; chase re-paths on the attack beat via
`CanToDoAttack`; cancel clears the target and stops walking.

### 1.5 Drunk stagger on the shared `Go` path (N1)
Decompile: `cract.cc:392-413`.
- [ ] Replace TFS drunk formula in `try_drunk_walk_direction` (`walk/mod.rs:116`) with the CipSoft
      probability `rand()%max(7-DrunkLevel,1)==0`, gated on `SKILL_DRUNKEN TimerValue>0 && Get()==0`
      (put the constants in `MechanicsProfile`).
- [ ] On stagger: `ToDoClear()` (abort remaining autowalk) + snapback (player) + `ToDoTalk("Hicks!")`
      + `ToDoStart()`, then still attempt the random step this beat.

**Acceptance:** a drunk 772 player mid-autowalk staggers to a random tile, says "Hicks!", and the
rest of the queued path is dropped.

### 1.6 Reference/comment cleanup (N2, N3, N4)
- [ ] Re-point 772 comments from `gameserver/src/…` to decompile refs (`receiving.cc`, `cract.cc`).
- [ ] Delete the "floor ×2" waypoint comment in `walk_timing.rs` (decompile only has diagonal ×3,
      `cract.cc:1526-1528`).
- [ ] Confirm `items_db.ground_speed_for_item` returns the CipSoft BANK `WAYPOINTS` value on 772
      (`cract.cc:1513-1522`).

### 1.7 Tests (772)
- [ ] Extend `creature_todo` / `idle_stimulus` tests to cover **players** (walk, autowalk, blocked
      step → clear+idle, attack/follow chase, drunk stagger).
- [ ] Retire/replace 772-player assertions that assumed `walk_queue` + `add_event_walk`.

**Phase 1 done when:** no 772 player walk/chase path uses `walk_queue`/`add_event_walk`/`clear_todo_772`;
`cargo test -p tfs-rust-core` green; behavioral checks in `tasks/player-walk-audit.md` §Verification
pass for 772.

> **Progress (2026-07-01):** 1.1–1.3 complete. 772 player walk/autowalk/stop/blocked-step now route
> through the unified ToDo engine. `cargo test` green (457 passed). Remaining: 1.4 (attack/follow),
> 1.5 (drunk), 1.6 (comments), 1.7 (tests). The 772 player walk path no longer uses
> `add_event_walk` or `clear_todo_772`; `walk_queue` is still used as the step source for `Go`
> actions (same as monsters — `on_walk` pops from `walk_queue`).

---

## Phase 2 — 1098 unification *(deferred; do not start until Phase 1 is proven)*

High-level only for now:
1. Implement the continuous-`Instant` side of the Phase 0 clock seam.
2. Route 1098 players + monsters through ToDo/`IdleStimulus`; replace
   `schedule_walk_followup_deadline` / `commit_next_walk_deadline` / `process_walk_deadlines`,
   `go_to_follow_creature`, `onThink` follow polling, and the `onCreatureMove` follow re-path.
3. **P8/P9 dissolve:** real-time follow re-path becomes `CanToDoAttack` on the attack beat; the
   `nextAction` lockout becomes `EarliestWalkTime` in the ToDo delay. Remove the 1098-only
   follow-dispatch and failed-move `nextAction` code.
4. Keep the 1098 **log** step-duration + clamped speed in `MechanicsProfile` (unchanged).
5. **QA gate:** validate 1098 step-timing/feel + client prediction against a live 10.98 client
   before removing the old scheduler. This is the one risk to confirm, not assume.

---

## Cross-cutting verification

```bash
rtk cargo check
rtk cargo clippy
rtk cargo test -p tfs-rust-core
```
Watch: `step_speed_tests` (`walk/mod.rs`), `walk_action.rs`, `creature_todo.rs`, `idle_stimulus.rs`.
