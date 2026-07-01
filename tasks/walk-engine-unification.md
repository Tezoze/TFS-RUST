# Walk Engine Unification — Migration Plan

**Date:** 2026-07-01 (updated 2026-07-02)
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
| **Phase 1.2** | ✅ DONE | `player_idle_stimulus` arm added; `request_idle_stimulus` widened to players. Player `Attack` execute wired in 1.4. |
| **Phase 1.3** | ✅ DONE | `on_walk` blocked-step handling widened to all ToDo creatures; unconditional `ToDoClear` + `IdleStimulus`; player-only `force_update_follow_path` branch removed. |
| **Phase 1.4** | ✅ DONE | Attack/follow/cancel packets → `SetAttackDest` + `CanToDoAttack` chase via `player_combat.rs`; `encode_clear_target` (`0xA3`) on codec; `FightModes` arm. |
| **Phase 1.5** | ✅ DONE | Drunk stagger (CipSoft formula + `CreatureAction::Talk` + `ToDoClear` + snapback + "Hicks!"). |
| **Phase 1.6** | ✅ DONE | Reference/comment cleanup (N2, N3, N4); "floor ×2" deleted; `ground_speed_for_item` verified. |
| **Phase 1.7** | ✅ DONE | Player ToDo/idle tests (11 new `test_phase1_*`; 468 total pass). |
| **Phase 2** | ⬜ DEFERRED | 1098 unification. |

**Verification (2026-07-02):** `cargo check` 0 errors · `cargo clippy` 0 new warnings in changed
files (pre-existing warnings in `tfs-rust-content` only) · `cargo test -p tfs-rust-core` 468 passed,
2 ignored, 0 failed · `cargo test -p tfs-rust-net` 95 passed, 0 failed.

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
| Wire bytes | — | `0x6B`/`0xB5`/`0x6D`/`0xA3` stay in `ProtocolCodec` |

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

## Phase 1 — 772 PLAYER PARITY *(complete)*

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
      `ToDoWait(1000)` path is handled in `player_execute_attack` (1.4).
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
(`SetAttackDest` / `CanToDoAttack`), `StopAttack` `:513-522`. Wire:
`gameserver/src/protocolgame.cpp:1485-1490` (`sendCancelTarget` `0xA3`).
- [x] `encode_clear_target` (`0xA3`) added to `ProtocolCodec` trait + `delegate_codec!` +
      `Codec772`/`Codec1098`/`Codec` impls (`tfs-rust-net/src/codec/`). Single-byte packet on
      both eras.
- [x] New `player_combat.rs` (`tfs-rust-core/src/`):
      - `player_set_attack_dest(conn, cid, target_wire_id, follow)` — C++ `SetAttackDest` +
        `CAttack` body: early-out on same target+follow; `target_id==0`/self/missing →
        `StopAttack`; validate (NPC → `AttackNotAllowed`, PZ → `ProtectionZone`, distance > 8
        or cross-floor → `TargetLost`, invisible creature → `TargetLost`); on success set
        `attack_target` (+ `follow_target` when following), `ChaseMode::Close` when following,
        `enqueue_creature_attack` + `todo_start_from_action`. On thrown `RESULT`: `ToDoClear` +
        `SendResult` (unless `NoError`/`NoWay`) + `ToDoYield` (`receiving.cc:1149-1153`).
      - `player_stop_attack(conn, cid)` — `StopAttack(0)`: clear `attack_target` +
        `follow_target`, send `0xA3` if was attacking.
      - `player_cancel_attack_and_follow(conn, cid)` — `SetAttackDest(0)` + `ToDoStop`:
        `player_stop_attack` + `player_todo_clear`.
      - `player_can_to_do_attack_chase(cid)` — `CanToDoAttack`: no target → `NoTarget`; target
        gone/dead → `TargetLost`; cross-floor or cheb > 8 → `TargetLost`; `Following ⇒ CLOSE`;
        `CLOSE & cheb>1` → repath via `get_creature_path_to` (player-aware via
        `tile_query_add_player`), populate `walk_queue`, `enqueue_creature_go_at(front=true)`,
        `todo_start_go_delay`; `cheb≤1` → `Adjacent` (strike deferred).
      - `player_execute_attack(cid)` — `TDAttack` execute for players: routes through
        `player_can_to_do_attack_chase`; `ChaseArmed` → re-queue `Attack` behind `Go`;
        `NoPath`/`Adjacent` → re-arm on attack beat (`delay_attack_ms(200)`); `TargetLost` →
        `StopAttack` + `SendClearTarget` + `ToDoClear` + `SendResult(TARGETLOST)` +
        `ToDoWait(1000)` + `ToDoStart` (`crplayer.cc:393-402`).
      - `creature_by_wire_id(wire_id)` — reverse wire-id → `CreatureId` lookup (players via
        `player_by_guid`; monsters/NPCs via low 32 bits of SlotMap key).
- [x] `game_loop.rs`: explicit `Attack`/`Follow`/`CancelAttackAndFollow`/`FightModes` arms
      (removed from the `_ =>` catch-all). Attack = `follow=false`, Follow = `follow=true`.
      `FightModes` sets `base.chase_mode` from `raw_chase_mode` (0→None, 1→Close; 772 only
      accepts NONE/CLOSE per `crcombat.cc:340`, out-of-range clamps to None with
      `tracing::warn`). Does not override `Close` forced by an active follow.
- [x] `idle_stimulus.rs` `execute_creature_todo_action` `Attack` arm: player branch routes to
      `player_execute_attack` (returns `AttackDeferred`); monster branch unchanged.
- [x] Note: 772 has **no** separate `follow_target`; follow == attack-with-`Following`. The Rust
      port sets `follow_target` only when `follow=true` so the shared `Go`/pathfinding arms key
      off it for repath. No TFS `goToFollowCreature` was added.

**Acceptance:** a 772 player can attack and follow a creature; chase re-paths on the attack beat via
`CanToDoAttack`; cancel clears the target and stops walking. **Player melee strike is deferred**
(no weapon-combat system yet); the chase still ticks via `delay_attack_ms(200)` re-arm.

### 1.5 Drunk stagger on the shared `Go` path (N1)
Decompile: `cract.cc:392-413`.
- [x] Replaced TFS drunk formula in `try_drunk_walk_direction` (`walk/mod.rs`) with the CipSoft
      probability `StaggerChance = max(7 - DrunkLevel, 1); rand() % StaggerChance == 0`, gated on
      `has_drunk_condition` (active `ConditionType::Drunk` or `drunkenness > 0`). `DrunkLevel`
      maps to `base.drunkenness` (set by `SpellImpact::Drunk`). The CipSoft `Get() == 0` skill
      check is implicitly true (no CipSoft skill system in Rust).
- [x] On stagger: `ToDoClear()` + `SendSnapback` (player, via `player_todo_clear_with_snapback`)
      + `ToDoTalk("Hicks!")` + `ToDoStart()`, then the random cardinal step replaces the
      intended direction (still calls `internal_move_creature_step` with the staggered dir).
- [x] `CreatureAction::Talk { text: &'static str }` variant added to `creature_todo.rs`
      (`cract.cc:848`, `:1367-1390`) + `enqueue_creature_talk` + `has_talk` helper. `&'static str`
      avoids allocation in the hot walk path.
- [x] `Talk` execute arm in `idle_stimulus.rs`: broadcasts via `broadcast_creature_say_viewport`
      with `SpeakType::Say` (players) or `SpeakType::MonsterSay` (monsters) — `cract.cc:409`.
- [x] Removed the inline `broadcast_creature_say_viewport` "Hicks!" call from `on_walk` (now via
      `ToDoTalk`). `SpeakType` import dropped from `walk/mod.rs`.

**Acceptance:** a drunk 772 player mid-autowalk staggers to a random tile, says "Hicks!", and the
rest of the queued path is dropped.

### 1.6 Reference/comment cleanup (N2, N3, N4)
- [x] Deleted "or 2 (floor)" from `walk_timing.rs:198` doc comment — decompile only has diagonal
      ×3 (`cract.cc:1526-1528`). Updated `waypoint_step_cost_for_direction` doc to cite
      `cract.cc:1526-1528` instead of the generic `cract.cc:1454, creature.cpp`.
- [x] 772 mechanics comments in `player_combat.rs` cite decompile refs (`crcombat.cc`,
      `receiving.cc`, `crplayer.cc`, `cract.cc`). Wire-only comments in `codec/v772.rs` cite
      `gameserver/src/protocolgame.cpp` (per `TFS-wire-codec` rule).
- [x] Confirmed `items_db.ground_speed_for_item` returns the BANK `WAYPOINTS` value on 772 —
      `objects.srv` overlay applied in `sim_harness.rs:583-588` (`overlay_otb_speeds_from_objects_srv`
      updates `db.items` which `ground_speed_for_item` reads). No change needed.

### 1.7 Tests (772)
- [x] 11 new `test_phase1_*` tests in `idle_stimulus.rs` test module:
      - `test_phase1_player_single_step_walk_via_todo` — `Go` enqueued after `player_move_request`.
      - `test_phase1_player_stop_auto_walk_clears_todo` — `player_stop_auto_walk` clears `Go`.
      - `test_phase1_player_attack_sets_target_and_enqueues` — `Attack` sets `attack_target` + `Attack`.
      - `test_phase1_player_follow_sets_follow_and_close_chase` — `Follow` sets `follow_target` + `Close`.
      - `test_phase1_player_cancel_clears_target_and_sends_clear_target` — cancel clears + sends `0xA3`.
      - `test_phase1_player_chase_arms_go_when_target_far` — `CanToDoAttack` arms `Go` at cheb>1.
      - `test_phase1_player_chase_adjacent_when_target_close` — `CanToDoAttack` returns `Adjacent` at cheb=1.
      - `test_phase1_player_chase_target_lost_when_far` — `CanToDoAttack` returns `TargetLost` at cheb>8.
      - `test_phase1_player_stop_attack_sends_clear_target` — `player_stop_attack` sends `0xA3`.
      - `test_phase1_drunk_stagger_clears_todo_and_enqueues_talk` — stagger clears queue + broadcasts "Hicks!".
      - `test_phase1_talk_action_broadcasts` — `Talk` execute emits `0xAA` speech packet.
- [x] No 772-player assertions assumed `walk_queue` + `add_event_walk` (verified — 1.1–1.3
      already retired that path).

**Phase 1 done when:** no 772 player walk/chase path uses `walk_queue`/`add_event_walk`/`clear_todo_772`;
`cargo test -p tfs-rust-core` green; behavioral checks in `tasks/player-walk-audit.md` §Verification
pass for 772. — **All met (2026-07-02).**

> **Progress (2026-07-01):** 1.1–1.3 complete. 772 player walk/autowalk/stop/blocked-step now route
> through the unified ToDo engine. `cargo test` green (457 passed). The 772 player walk path no
> longer uses `add_event_walk` or `clear_todo_772`; `walk_queue` is still used as the step source
> for `Go` actions (same as monsters — `on_walk` pops from `walk_queue`).

> **Progress (2026-07-02):** Phase 1 **complete** (1.4–1.7). Attack/follow/cancel packets route
> through `SetAttackDest` + `CanToDoAttack` chase via `player_combat.rs`. Drunk stagger uses
> CipSoft formula (`max(7-DrunkLevel,1)`) with `CreatureAction::Talk` + `ToDoClear` + snapback.
> `encode_clear_target` (`0xA3`) added to codec. 11 new `test_phase1_*` tests; 468 total pass.
> Player melee **strike** is deferred (no weapon-combat system yet); chase re-paths on the attack
> beat. Phase 2 (1098 unification) is the next milestone.

---

## Known gaps (post-Phase 1)

- **Player melee strike** — `CanToDoAttack` chase re-paths on the attack beat, but no weapon
  damage is dealt. `player_execute_attack` returns `AttackDeferred` and re-arms via
  `delay_attack_ms(200)`. A "You are exhausted."-style message is **not** sent (no strike
  attempted). This closes when a player weapon-combat system lands.
- **PVP secure-mode / `IsAttackJustified`** — `CombatResult::SecureMode` is reserved but not
  emitted; `crcombat.cc:374-381` PVP checks and `CheckRight(NO_ATTACK/ATTACK_EVERYWHERE)` are
  deferred to the player weapon-combat system.
- **Player chase pathfinding terrain gate** — `get_creature_path_to` uses
  `creature_can_stand_for_pathfind` (matches `MovePossible` planning) but does **not** apply the
  772 TShortway terrain/BANK gate that `monster_tshortway_fill_walkable` applies. If chase
  quality differs from monsters, switch to a player-aware terrain fill in a follow-up.
- **`FightModes` fight mode / secure mode storage** — only `chase_mode` is wired; fight mode and
  secure mode bytes are read but not stored (deferred to player weapon-combat system).

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
rtk cargo test -p tfs-rust-net
```
Watch: `step_speed_tests` (`walk/mod.rs`), `walk_action.rs`, `creature_todo.rs`,
`idle_stimulus.rs` (`test_phase1_*`), `player_combat.rs` (via `idle_stimulus` tests).

---

## Files touched (Phase 1.4–1.7)

| File | Change |
|------|--------|
| `crates/tfs-rust-net/src/codec/mod.rs` | `encode_clear_target` on `ProtocolCodec` trait + `delegate_codec!` + `Codec` impl |
| `crates/tfs-rust-net/src/codec/v772.rs` | `encode_clear_target` impl (single byte `0xA3`) |
| `crates/tfs-rust-net/src/codec/v1098.rs` | `encode_clear_target` impl (single byte `0xA3`) |
| `crates/tfs-rust-core/src/player_combat.rs` | **New** — `SetAttackDest`/`StopAttack`/`CanToDoAttack` chase for players |
| `crates/tfs-rust-core/src/lib.rs` | Register `player_combat` module |
| `crates/tfs-rust-core/src/game_loop.rs` | `Attack`/`Follow`/`CancelAttackAndFollow`/`FightModes` arms |
| `crates/tfs-rust-core/src/walk/mod.rs` | CipSoft drunk formula + stagger path; `player_todo_clear` made `pub(crate)`; `SpeakType` import dropped |
| `crates/tfs-rust-core/src/walk/walk_timing.rs` | Deleted "floor ×2" comment; re-pointed to `cract.cc:1526-1528` |
| `crates/tfs-rust-core/src/idle_stimulus.rs` | Player `Attack` execute via `player_execute_attack`; `Talk` execute arm; `SpeakType` import; 11 new tests |
| `crates/tfs-rust-core/src/creature_todo.rs` | `CreatureAction::Talk` variant + `enqueue_creature_talk` + `has_talk` |
