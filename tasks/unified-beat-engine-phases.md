# Unified Beat Engine — Phased Implementation Plan

**Date:** 2026-07-02
**Status:** 🟨 IN PROGRESS — Phase 0 done, Phase 1 done, Phase 2 done, Phase 3 done, Phase 4 done, Phase 5 done.
**Strategy / rationale:** `tasks/unified-beat-engine-plan.md` (read first).
**Engine parity gaps to close first:** `docs/GAME_LOOP_772_AUDIT.md`.
**Walk sub-effort (subsumed here):** `tasks/walk-engine-unification.md` Phase 2.

## Objective

Make the beat-driven CipSoft ToDo engine the **single simulation engine** for every era. 772
runs on it today; this plan hardens it, then **deletes the separate 1098 AI/player logic** so 1098
runs on the **same 772 AI and player logic** (the uniform system). Per-era differences live
**only** in `MechanicsProfile` (+ `data/formulas/<v>.lua`) and `ProtocolCodec`. The
`GameWorld::beat_driven_loop` boolean is removed.

**Not behavior-preserving for 1098** — 1098 inherits the 772 AI/player behavior wholesale, gated
by the live-client parity check in Phase 9. Every step still gates on
`rtk cargo check && rtk cargo clippy --all-targets && rtk cargo test`.

**Hard constraint — do not edit the 772 system.** The 772 AI, player logic, ToDo engine,
`IdleStimulus`, walk/combat paths, etc. that are already built are **frozen**. Phases 3–5 only
**delete 1098-specific code** and **route 1098 onto the existing 772 paths unchanged**. No
refactor, rename, behavior tweak, or "improvement" to the 772 paths is permitted in this effort —
not even changes that look like cleanups. If a 1098-era outcome genuinely differs from 772, the
difference goes into `MechanicsProfile` / `data/formulas/<v>.lua`, never back into the 772 code.
The 772 test suite must stay byte-stable throughout.

---

## Phase ordering at a glance

| Phase | Title | Risk | Gate |
|-------|-------|------|------|
| 0 | Harden the canonical (772) engine | low/med | test-frozen (772 parity) |
| 1 | Branch inventory (classify all 186 sites) | ~zero | doc only, no code |
| 2 | Decouple beat-size + cadence + flush from era | low | test-frozen |
| 3 | Delete 1098 **monster** AI; 1098 reuses 772 monster AI | med | 1098 harness spot-checks |
| 4 | Delete 1098 **player** logic; 1098 reuses 772 player logic | high | 1098 harness spot-checks |
| 5 | Retire 1098 reactive machinery (delete) | med | grep-clean + green |
| 6 | Collapse `beat_driven_loop` flag | mechanical | zero prod hits |
| 7 | Single `run_game_loop` entry | low | one loop fn |
| 8 | Naming reconciliation (`_772` → canonical) | mechanical | audit P3 exit |
| 9 | **Parity QA gate** (772 stable + live 1098) | — | sign-off |

Do not start a phase until the prior one is green. Phase 9's 1098 sign-off blocks Phase 5's
final deletions from shipping.

---

## Phase 0 — Harden the canonical engine (prerequisite)

**Why first:** the beat engine cannot be the "source of truth" while it has open parity gaps.
Close the `docs/GAME_LOOP_772_AUDIT.md` findings, prioritizing the structural one (Finding 8)
that blocks "all mechanics run through ToDo."

- [x] **F1 — idle-timer exemption list** (`connections_772.rs::packet_counts_as_action_772`):
      exempt `Ping`, `StopAutoWalk`, `CancelAttackAndFollow`, `UpdateTile` (772 "refresh field",
      0xC9), `UpdateContainer`; **remove `Turn`** and **remove `PingBack`** (OTClient-only 0x1D).
      (`connections.cc:53-63`)
- [x] **F3 — fed regen from `vocations.xml`** (`process_skills.rs`): delete `fed_regen_cadence`
      hardcoded table; read `gainhpticks/amount` + `gainmanaticks/amount` from vocation def; add
      PZ gate + food-remaining gate (`SKILL_FED > 0`). (`crskill.cc:812-885`)
- [x] **F5 — lag log gate** (`game_world_tick.rs::advance_beat_772`): log the error only on the
      `!lag_772 → lag_772` transition. (`main.cc:447`)
- [x] **F2 — `ProcessCreatures` gaps** (`creature_think.rs`): PK-mark clearing on
      `EarliestLogoutRound` expiry (stub — field zeroed, full `ClearPlayerkillingMarks` deferred
      until PvP aggressor subsystem exists); PZ-gated HP+1/Mana+4 item regen separate from
      `TSkillFed`, keyed on `food_level` (`SKILL_FED` `Act`). (`crmain.cc:1087-1107`)
      **Food persistence:** added `food_remaining` + `food_level` columns to `players`
      (migration `20260702000000_food_skill.sql`); load/save in `player.rs`; offline food drain
      on login (`crplayer.cc:1395-1400`). **Eat action:** `player:feed(amount)` Lua binding
      (mutation `PlayerFeed` → `lua_script_player_feed`); `food.lua` updated to use
      `player:getFood()` instead of `CONDITION_REGENERATION` for the "full" check.
- [x] **F4 — Scheduler dispatch** (`scheduler.rs` / `game_loop.rs`): actually invoke
      `GameCommand::LuaCallback { event_id }` instead of only tracing it. Wire to `addEvent`/
      `stopEvent` per `tfs-lua-boundaries.md` §"Full API Port Plan" item 4.
- [x] **F8 — route player non-walk actions through ToDo `Execute`** (structural): move
      `player_use_item` / `player_use_item_ex` / `player_look_at` / container ops / `Say` off the
      reactive `handle_game_packet` path and onto `ToDoUse`/`ToDoMove`/`ToDoTrade` +
      `CalculateDelay` (`EarliestMultiuseTime`, `cract.cc:765-766`). This is the phase that makes
      "the ToDo engine is the sole mechanic" literally true for players.
      **Note:** F8 is large; it may be split into its own sub-plan. It is a hard prerequisite for
      calling the engine the single source of truth, but Phases 1–2 can proceed in parallel.

**Exit:** `docs/GAME_LOOP_772_AUDIT.md` findings resolved or explicitly deferred with rationale;
772 test suite green and behavior stable; player actions (not just walk) flow through ToDo.

---

## Phase 1 — Branch inventory (classify before editing)

**Deliverable:** `tasks/beat-unify-branch-inventory.md` — every `beat_driven_loop` site
(186 prod hits / 25 files) tagged with a fate. **No code changes.**

- [x] Generate the raw site list (`grep -rn beat_driven_loop crates/tfs-rust-core/src`).
- [x] Tag each site:
      - **U (Unify):** 1098 arm is dead post-migration → will delete `else`, keep beat arm.
      - **K (Profile knob):** genuine era-different *outcome* → move to `MechanicsProfile`.
      - **C (Clock adapter):** only wall-clock vs `server_ms` → route through the clock seam.
      - **X (Codec/transport):** belongs in `tfs-rust-net`.
- [x] Review all **K** rows with the user — these are the only real era differences that survive.

**Exit:** every site has a fate + target; K rows signed off.

---

## Phase 2 — Decouple beat size / cadence / flush from era

**Goal:** the beat engine runs at *any* beat, cadence, and flush policy so 1098 can adopt it
without inheriting 200 ms / staggered-1000 ms think / beat-end-only flush.

- [x] Confirm `beat_ms` / `step_beat_ms` / `step_speed` remain independent (already true).
- [x] Add explicit profile fields for the era splits Phase 1 surfaced as **K**:
      - **think cadence** (772 staggered ~1000 ms vs 1098 50 ms bucketed),
      - **condition/skill tick interval**,
      - **flush policy** (beat-end vs immediate-on-movement).
- [x] Have the loop + `advance_beat` read cadence/flush/beat from the profile, not from
      `beat_driven_loop`.
- [x] 772 and 1098 behavior unchanged this phase (pure plumbing).

**Exit:** instantiating the beat loop with `beat_ms=50` + immediate-ish flush + 50 ms think
produces a "1098-flavored" run in a dev harness (not yet the default).

---

## Phase 3 — Delete 1098 monster AI; 1098 reuses the 772 monster AI ✅ DONE

Do monsters before players (lower visibility, easier rollback).

**Decision:** there is no separate 1098 monster AI to migrate. The 1098 monster AI is **deleted**;
1098 monsters run on the **same 772 monster AI** (already ToDo/`IdleStimulus`-based). Per-era AI
knobs (chase radius, flee thresholds, kite cadence, etc.) live in `MechanicsProfile` /
`data/formulas/<v>.lua` — not in a parallel AI path.

**Hard constraint:** the 772 monster AI is **frozen**. This phase only **deletes 1098-specific
monster code** and routes 1098 monsters onto the existing 772 path **unchanged**. No edits to the
772 AI functions, no renames, no cleanups. If a 1098 monster outcome genuinely differs from 772,
the difference goes into `MechanicsProfile` — never back into the 772 AI code. 772 monster tests
must stay byte-stable.

- [x] Implement the small-beat side of the Phase 0 clock seam so `server_ms` drives 1098
      scheduling. (`on_tick` now advances `server_ms` by `beat_ms` + drains ToDo queue;
      `now_ms()` returns `server_ms` unconditionally.)
- [x] Delete the 1098 monster AI code path: `go_to_follow_creature` 1098 fallthrough +
      `onThink` follow poll + `onCreatureMove` re-path + `monster_on_think_target` +
      `monster_maybe_walk_to_spawn` + 1098 distance-step/dance/reconcile + 1098 push +
      1098 `ai_rng` spell path + 1098 synchronous target search + 1098 Z-change clear +
      1098 drawblood duplicate. **P8 dissolves.** ~700 lines deleted.
- [x] Remove `beat_driven_loop`-gated `creature_uses_todo_execute` / `request_idle_stimulus` /
      idle-stimulus arms that special-case 1098 monsters — `creature_uses_todo_execute` returns
      `true` for monsters regardless of `beat_driven_loop`; 1098 monsters take the same ToDo path
      as 772 monsters unconditionally.
- [x] Audit monster AI for any `if version == 1098` / era-branched outcome — none found; genuine
      era differences already in `MechanicsProfile` (`beat_ms`, `step_speed`, `target_distance`).
- [x] Keep `beat_driven_loop` *temporarily forced true for 1098* behind `TFS_FORCE_BEAT_LOOP=1`
      env var in `run_server.rs` so the deletion is A/B-comparable until Phase 9 signs off.
- [x] Verify the 772 monster test suite is byte-stable — 568 core tests pass, 0 failures.
      7 1098-specific tests deleted, 6 rewritten to 772 setup (`beat_driven_test_world` +
      `advance_beat_772`/`drain_todo_queue`).

**Exit:** no 1098-specific monster AI code remains; 1098 monsters walk/chase/kite/flee via the
single ToDo/`IdleStimulus` path; `772_MONSTER_AI_AUDIT`-style spot checks pass for both eras
(differences only via `MechanicsProfile`). Committed as `1472f02`.

---

## Phase 4 — Delete 1098 player logic; 1098 reuses the 772 player logic ✅ DONE

Mirror the completed 772 player work (`player_combat.rs`, `walk/mod.rs`, `player_move_request`).

**Decision:** there is no separate 1098 player path to migrate. The 1098 player logic is
**deleted**; 1098 players run on the **same 772 player logic** (already ToDo-based:
`ToDoClear`(+snapback) → `TDGo` → `ToDoStart`, `SetAttackDest` + `CanToDoAttack` chase,
`EarliestWalkTime` lockout). Per-era player knobs (walk beat, step curve, attack cadence, etc.)
live in `MechanicsProfile` / `data/formulas/<v>.lua` — not in a parallel player path.

**Hard constraint:** the 772 player logic is **frozen**. This phase only **deletes 1098-specific
player code** and routes 1098 players onto the existing 772 path **unchanged**. No edits to the
772 player/walk/combat functions, no renames, no cleanups. If a 1098 player outcome genuinely
differs from 772, the difference goes into `MechanicsProfile` — never back into the 772 player
code. 772 player tests must stay byte-stable.

- [x] Delete the 1098 player walk/autowalk/stop path; 1098 players take the 772
      `ToDoClear`(+snapback) → `TDGo` → `ToDoStart` path unconditionally.
- [x] Delete the 1098 attack/follow/cancel path; 1098 players take the 772 `SetAttackDest` +
      `CanToDoAttack` chase path unconditionally.
- [x] Delete the 1098 `nextAction` lockout on failed move; the 772 `EarliestWalkTime` ToDo delay
      applies to both eras. **P9 dissolves.**
- [x] Audit player code for any `if version == 1098` / era-branched outcome; genuine era
      differences move to `MechanicsProfile` (re-classify per Phase 1 as **K** rows), everything
      else collapses to the single 772 path.
- [x] Player non-walk actions already unified in Phase 0/F8 — verify they run for 1098 too
      (single path, no 1098 fork).
- [x] Keep `beat_driven_loop` *temporarily forced true for 1098* behind `TFS_FORCE_BEAT_LOOP=1`
      dev flag so the deletion is A/B-comparable until Phase 9 signs off.
- [x] Verify the 772 player test suite is byte-stable before and after this phase
      (`rtk cargo test -p tfs-rust-core` walk/player_combat suites).

**Exit:** no 1098-specific player code remains; a 1098 player walks/attacks/follows/uses via the
single 772 ToDo path in a harness; single-beat move latency verified; differences only via
`MechanicsProfile`; 772 player tests byte-stable. 567 core tests pass, 0 failures.

**Files changed:**
- `player_combat.rs` — deleted 1098 defer guards in `player_set_attack_dest` / `player_cancel_attack_and_follow`.
- `walk/mod.rs` — deleted 1098 arms in `player_move_request`, `player_auto_walk_path`,
  `player_stop_auto_walk`, `player_todo_clear_with_snapback`, `todo_start_go_delay`,
  `on_walk` (walk_delay, walk_destinations pop, notify_go_ms, last_step_server_ms, reschedule,
  nextAction lockout).
- `walk_action.rs` — `on_player_walk_complete` + `process_walk_action_tasks` → no-ops;
  `defer_player_walk_action` 1098 arm deleted.
- `game_loop.rs` — deleted 1098 reactive `else` arms for Throw/UseItem/UseItemEx/RotateItem;
  `process_walk_deadlines` call deleted; `player_reset_connection_rounds` made unconditional.
- `game_world.rs` — deleted 1098 `nextAction` arms in `player_packet_action_ready`,
  `player_walk_action_ready`, `player_use_item_ready`, `player_use_item_ex_ready`,
  `player_apply_multiuse_exhaust`, `player_apply_spell_exhaust`.
- `connections_772.rs` — deleted 1098 defer guards in `player_reset_connection_rounds`,
  `process_connections_772`, `tick_ambiente_light_772`.
- `spell.rs` — deleted 1098 `nextAction` arm in `can_cast_instant`.
- `process_skills.rs` — `process_player_fed_regen_772` made unconditional.
- `creature_todo.rs` — `creature_todo_yield` 1098 guard deleted; `idle_enqueue_paced_go` 1098 arm deleted.
- `idle_stimulus.rs` — updated Phase 4 comment (player gating unchanged, env var drives it).
- Tests: `rotate_item_no_op_when_not_beat_driven` → `rotate_item_enqueues_even_when_not_beat_driven`;
  `can_cast_instant_blocks_while_next_action_in_future` deleted (1098 `nextAction` path gone).

---

## Phase 5 — Retire 1098 reactive machinery (delete, don't gate) ✅ DONE

After Phases 3–4 the 1098 AI and player logic is gone; 1098 runs on the 772 paths. This phase
deletes the **reactive machinery** those paths used to dispatch on 1098 — the walk-wake /
deadline / dual-path plumbing that the uniform ToDo engine replaces. **Ship-gated by Phase 9
sign-off.**

**Hard constraint (same as Phases 3–4):** the 772 paths are **frozen**. This phase only deletes
1098-only reactive plumbing. No edits to the 772 ToDo/walk/combat/AI functions. If a deletion
would touch a shared function, stop and escalate — do not refactor the 772 side to "make it
work" for 1098.

- [x] `run_game_loop_1098` walk-wake branch; `walk_wake_tx`/`walk_wake_rx`; `sleep_until` walk
      scheduling; `GameWorld::walk_wake_tx` field. Also deleted the `run_game_loop` back-compat
      alias and the `TFS_FORCE_BEAT_LOOP=1` dev flag — `run_server.rs` unconditionally calls
      `run_game_loop_772` and forces `beat_driven_loop = true`.
- [x] `process_walk_deadlines`, `schedule_walk_followup_deadline` (collapsed to ToDo-only),
      `commit_next_walk_deadline`, `sync_walk_timer_arm`, `process_walk_due_from_wake`,
      `check_creature_walk` (wake-from-deadline entry). The `schedule_walk_followup_deadline`
      name is retained but its body is now ToDo-only (no `Instant`/`commit` arm).
- [x] `walk_action_due` dual path → single ToDo path. Deleted the `walk_action_due` field from
      `Player`, plus `process_walk_action_tasks`, `on_player_walk_complete`, and
      `run_player_walk_action` (all dead no-ops / unreachable after Phase 4). `walk_action`
      remains as a deferred-action marker cleared by `ToDoClear` (audit #3).
- [x] `Instant`-based `walk_timer` / `next_walk_check` on `CreatureBase` → `next_wakeup` only.
      Deleted the `WalkTimer` newtype + its `Deref`/`DerefMut`/`Clone` impls and the
      `WALK_DEADLINE_GRACE` constant. `walk_timer_idle()` simplified to `&self` (no
      `beat_driven_loop` arg).
- [x] `FlushPolicy::ImmediateOnMovement` → profile-driven flush (Phase 2 knob). Deleted the
      `FlushPolicy` enum, `needs_immediate_flush`, `packet_would_immediate_flush`, and the
      `flush_policy` parameter from `handle_game_packet` / `dispatch_command` /
      `run_game_loop_772`. Both eras use beat-end `SendAll`.
- [x] Migrate tests off `walk_wake_tx = None` / `process_walk_deadlines()` to the ToDo path
      (rewrite assertions, don't tweak to pass). ~40 `world.walk_wake_tx = None;` lines removed
      across 5 test files; `process_walk_deadlines()` calls replaced with
      `schedule_creature_wakeup` + `drain_todo_queue`; `next_walk_check`/`walk_timer`/
      `walk_action_due` field initializations removed; F8 S7 tests for `on_player_walk_complete`
      deleted (function gone); `beat_driven_walk_schedules_todo_queue_not_tokio` now checks
      `next_wakeup` instead of `walk_timer`.
- [x] Confirm the monster follow machinery (`go_to_follow_creature` + `onThink` follow poll +
      `onCreatureMove` re-path) deleted in Phase 3 has no remaining references. `grep` for
      `walk_wake_tx`/`process_walk_deadlines`/`commit_next_walk_deadline`/`next_walk_check`/
      `walk_timer`/`check_creature_walk`/`process_walk_due_from_wake` in `monster_ai.rs` returns
      only comment references — no live code.

**Exit:** `grep` for the above symbols returns only comment/doc references; both eras run on the
single 772-based engine. 564 core tests pass, 0 failures. Workspace `cargo check` clean.

**Files changed:**
- `game_loop.rs` — deleted `FlushPolicy`, `needs_immediate_flush`, `packet_would_immediate_flush`,
  `run_game_loop_1098`, `run_game_loop` alias; dropped `flush_policy` param from `handle_game_packet` /
  `dispatch_command` / `run_game_loop_772`; deleted `movement_packets_flush_immediately_on_1098` test.
- `game_world.rs` — removed `walk_wake_tx` field + `UnboundedSender` import; `GameWorld::new` no
  longer takes the `walk_wake_tx` param.
- `run_server.rs` — unconditional `run_game_loop_772`; `beat_driven_loop = true` forced; deleted
  `TFS_FORCE_BEAT_LOOP` env-var read + `walk_wake_tx`/`walk_wake_rx` channel setup.
- `game_world_tick.rs` — `on_tick` no longer calls `process_walk_deadlines` /
  `process_walk_action_tasks`.
- `walk/mod.rs` — deleted `commit_next_walk_deadline`, `sync_walk_timer_arm`,
  `process_walk_due_from_wake`, `check_creature_walk`, `process_walk_deadlines`,
  `WALK_DEADLINE_GRACE`; collapsed `add_event_walk` / `schedule_walk_followup_deadline` /
  `player_move_request` / `player_auto_walk_path` / `player_stop_auto_walk` / `on_walk` chase
  branch to ToDo-only (removed `if beat_driven_loop` guards + 1098 `commit` else-arms);
  `add_event_walk` lost the `scheduling_base: Instant` param; `stop_event_walk` clears
  `next_wakeup` only; `schedule_creature_wakeup` no longer touches `next_walk_check`.
- `walk_action.rs` — deleted `process_walk_action_tasks`, `on_player_walk_complete`,
  `run_player_walk_action`; `clear_player_walk_action` / `defer_player_walk_action` /
  `set_next_walk_action_task` no longer touch `walk_action_due`.
- `creature/base.rs` — deleted `WalkTimer` newtype (+ `Deref`/`DerefMut`/`Clone`),
  `next_walk_check` field, `walk_timer` field; `walk_timer_idle()` takes no args.
- `creature/player.rs` — deleted `walk_action_due` field.
- `creature/mod.rs` — removed `WalkTimer` re-export.
- `lib.rs` — `pub use game_loop::{graceful_shutdown, run_game_loop_772, wait_for_shutdown_signal}`
  (dropped `run_game_loop`, `run_game_loop_1098`).
- `sim_harness.rs` — `GameWorld::new` callers updated; `beat_driven_walk_schedules_todo_queue_not_tokio`
  test checks `next_wakeup`; `process_walk_deadlines()` call replaced with `drain_todo_queue`.
- `idle_stimulus.rs` — `walk_timer_idle()` calls updated; `TFS_FORCE_BEAT_LOOP` comment updated;
  removed unused `TargetSearchType` import.
- `monster_ai.rs` — `walk_timer_idle()` calls updated.
- Tests (`monster_ai_world_tests`, `monster_ai_tests`, `monster_push_tests`,
  `creature_think_tests`, `idle_stimulus_tests`, `arena.rs`) — removed `walk_wake_tx = None`,
  `next_walk_check`/`walk_timer`/`walk_action_due` field initializations, `process_walk_deadlines`
  calls, F8 S7 `on_player_walk_complete` tests.

---

## Phase 6 — Collapse the `beat_driven_loop` flag

Walk the Phase 1 inventory:

- [ ] **U** rows: delete `else`, keep beat arm unconditionally.
- [ ] **K** rows: replace boolean with the profile read.
- [ ] **C** rows: route through the clock seam.
- [ ] **X** rows: push to codec.
- [ ] Remove `GameWorld::beat_driven_loop` + constructor wiring.
- [ ] `now_ms()` unconditionally returns `server_ms`.

**Exit:** `grep -rn "beat_driven_loop" crates/tfs-rust-core/src` → **zero** production hits.

---

## Phase 7 — Single loop entry point

- [ ] Merge `run_game_loop_772` into `run_game_loop` (Phase 5 already deleted
      `run_game_loop_1098` + the back-compat alias). `run_server.rs` reads beat/cadence/flush
      from the profile — no `if beat_driven` fork.
- [ ] Rewrite `docs/GAME_LOOP_ARCHITECTURE.md`: one beat engine, per-era beat size + cadence via
      profile; keep the C++ reference index (both eras still cite sources).

**Exit:** one loop function; no loop-selection branch in `run_server.rs`.

---

## Phase 8 — Naming reconciliation (REFACTOR_AUDIT Phase 3)

Under unification these are the *canonical* functions, not a "772 variant."

- [ ] `advance_beat_772` → `advance`, `process_creatures_772` → `process_creatures`,
      `process_connections_772` → `process_connections`, `tick_ambiente_light_772` →
      `tick_ambient_light`, `subsystem_counters_772` → `subsystem_counters`, etc.
- [ ] Recommended sequence: **unification (Phases 0–7) first, then this rename**, so names reflect
      the final single-engine reality. If REFACTOR_AUDIT Phase 3 already ran with `*_beat`
      suffixes, re-simplify here.

**Exit:** `grep -rn "fn .*_772" crates/tfs-rust-core/src` returns only config/test items.

---

## Phase 9 — Parity QA gate (mandatory before ship)

- [ ] **772:** `tasks/player-walk-audit.md` §Verification + `chase_kite_sim` harness — outcomes
      byte-stable vs current 772 (this era already uses the engine).
- [ ] **1098:** side-by-side vs the current `run_game_loop_1098` build on a **live 10.98 client** —
      walk/diagonal cadence, autowalk, follow re-path latency, monster chase/kite feel, attack
      beat, client-side prediction (no rubber-banding), light/ambient, ping.
- [ ] **Decide 1098 `beat_ms` empirically** here (50 ms vs larger). Record in
      `docs/GAME_LOOP_ARCHITECTURE.md`.
- [ ] If 1098 feel can't be recovered via `beat_ms`/cadence knobs → **stop and escalate**: add a
      profile-selected "continuous drain" scheduling mode on the same engine rather than forcing
      beat quantization. Do not silently ship a regression.

**Exit:** both eras signed off; Phase 5 deletions safe to ship.

---

## Adding a new era afterward (the payoff)

1. `Codec` impl in `tfs-rust-net/src/codec/` (wire bytes / opcodes / caps).
2. `MechanicsProfile::for_version(N)` + `data/formulas/N.lua` (beat, step curve, ticks, cadence,
   AI knobs).
3. **Zero core branches.** If a core `if version == …` is ever needed, the difference belongs in
   the profile or codec — re-classify per Phase 1.

## Verification (every step)

```bash
rtk cargo check
rtk cargo clippy --all-targets
rtk cargo test -p tfs-rust-core
rtk cargo test -p tfs-rust-net
```
Watch suites: `idle_stimulus` (`test_phase1_*`), `creature_todo`, `walk/mod` step-speed,
`monster_ai_world_tests`, `subsystem_counters_772`, `game_world_tick`.
