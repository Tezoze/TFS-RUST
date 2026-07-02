# Unified Beat Engine — Phased Implementation Plan

**Date:** 2026-07-02
**Status:** 🟨 IN PROGRESS — Phase 0 (F1 done).
**Strategy / rationale:** `tasks/unified-beat-engine-plan.md` (read first).
**Engine parity gaps to close first:** `docs/GAME_LOOP_772_AUDIT.md`.
**Walk sub-effort (subsumed here):** `tasks/walk-engine-unification.md` Phase 2.

## Objective

Make the beat-driven CipSoft ToDo engine the **single simulation engine** for every era. 772
runs on it today; this plan hardens it, then moves 1098 (and any future codec) onto it. Per-era
differences live **only** in `MechanicsProfile` (+ `data/formulas/<v>.lua`) and `ProtocolCodec`.
The `GameWorld::beat_driven_loop` boolean is removed.

**Not behavior-preserving for 1098** — gated by the live-client parity check in Phase 9. Every
step still gates on `rtk cargo check && rtk cargo clippy --all-targets && rtk cargo test`.

---

## Phase ordering at a glance

| Phase | Title | Risk | Gate |
|-------|-------|------|------|
| 0 | Harden the canonical (772) engine | low/med | test-frozen (772 parity) |
| 1 | Branch inventory (classify all 186 sites) | ~zero | doc only, no code |
| 2 | Decouple beat-size + cadence + flush from era | low | test-frozen |
| 3 | Route 1098 **monsters** onto ToDo/`IdleStimulus` | med | 1098 harness spot-checks |
| 4 | Route 1098 **players** onto ToDo | high | 1098 harness spot-checks |
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
- [ ] **F8 — route player non-walk actions through ToDo `Execute`** (structural): move
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

- [ ] Generate the raw site list (`grep -rn beat_driven_loop crates/tfs-rust-core/src`).
- [ ] Tag each site:
      - **U (Unify):** 1098 arm is dead post-migration → will delete `else`, keep beat arm.
      - **K (Profile knob):** genuine era-different *outcome* → move to `MechanicsProfile`.
      - **C (Clock adapter):** only wall-clock vs `server_ms` → route through the clock seam.
      - **X (Codec/transport):** belongs in `tfs-rust-net`.
- [ ] Review all **K** rows with the user — these are the only real era differences that survive.

**Exit:** every site has a fate + target; K rows signed off.

---

## Phase 2 — Decouple beat size / cadence / flush from era

**Goal:** the beat engine runs at *any* beat, cadence, and flush policy so 1098 can adopt it
without inheriting 200 ms / staggered-1000 ms think / beat-end-only flush.

- [ ] Confirm `beat_ms` / `step_beat_ms` / `step_speed` remain independent (already true).
- [ ] Add explicit profile fields for the era splits Phase 1 surfaced as **K**:
      - **think cadence** (772 staggered ~1000 ms vs 1098 50 ms bucketed),
      - **condition/skill tick interval**,
      - **flush policy** (beat-end vs immediate-on-movement).
- [ ] Have the loop + `advance_beat` read cadence/flush/beat from the profile, not from
      `beat_driven_loop`.
- [ ] 772 and 1098 behavior unchanged this phase (pure plumbing).

**Exit:** instantiating the beat loop with `beat_ms=50` + immediate-ish flush + 50 ms think
produces a "1098-flavored" run in a dev harness (not yet the default).

---

## Phase 3 — Route 1098 monsters onto ToDo/`IdleStimulus`

Do monsters before players (lower visibility, easier rollback).

- [ ] Implement the small-beat side of the Phase 0 clock seam so `server_ms` drives 1098
      scheduling.
- [ ] Flip the `creature_uses_todo_execute` / `request_idle_stimulus` / idle-stimulus arms that
      are `beat_driven_loop`-gated to include 1098 (many are already
      `creature_uses_todo_execute`-based — they just need the flag true for 1098).
- [ ] Replace 1098 monster follow (`go_to_follow_creature` + `onThink` follow poll +
      `onCreatureMove` re-path) with `CanToDoAttack` on the attack beat. **P8 dissolves.**
- [ ] Keep `beat_driven_loop` *temporarily forced true for 1098* behind a dev flag so both paths
      remain A/B-comparable.

**Exit:** 1098 monsters walk/chase/kite/flee via ToDo in a harness; `772_MONSTER_AI_AUDIT`-style
spot checks pass.

---

## Phase 4 — Route 1098 players onto ToDo

Mirror the completed 772 player work (`player_combat.rs`, `walk/mod.rs`, `player_move_request`).

- [ ] 1098 walk/autowalk/stop → `ToDoClear`(+snapback) → `TDGo` → `ToDoStart` (same as 772).
- [ ] 1098 attack/follow/cancel → `SetAttackDest` + `CanToDoAttack` chase.
- [ ] `nextAction` lockout on failed move → `EarliestWalkTime` in the ToDo delay. **P9 dissolves.**
- [ ] Player non-walk actions already unified in Phase 0/F8 — verify they run for 1098 too.

**Exit:** a 1098 player walks/attacks/follows/uses via ToDo in a harness; single-beat move
latency verified.

---

## Phase 5 — Retire 1098 reactive machinery (delete, don't gate)

After Phases 3–4 nothing should call these on 1098. **Ship-gated by Phase 9 sign-off.**

- [ ] `run_game_loop_1098` walk-wake branch; `walk_wake_tx`/`walk_wake_rx`; `sleep_until` walk
      scheduling; `GameWorld::walk_wake_tx` field.
- [ ] `process_walk_deadlines`, `schedule_walk_followup_deadline`, `commit_next_walk_deadline`.
- [ ] `go_to_follow_creature` + `onThink` follow poll + `onCreatureMove` re-path.
- [ ] `walk_action_due` dual path → single ToDo path.
- [ ] `Instant`-based `walk_timer` / `next_walk_check` on `CreatureBase` → `next_wakeup` only.
- [ ] `FlushPolicy::ImmediateOnMovement` → profile-driven flush (Phase 2 knob).
- [ ] Migrate tests off `walk_wake_tx = None` / `process_walk_deadlines()` /
      `go_to_follow_creature()` to the ToDo path (rewrite assertions, don't tweak to pass).

**Exit:** `grep` for the above symbols returns only removed/renamed results; both eras run on the
shared engine.

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

- [ ] Merge `run_game_loop_772` into `run_game_loop`; delete `run_game_loop_1098` + the
      back-compat alias. `run_server.rs` reads beat/cadence/flush from the profile — no
      `if beat_driven` fork.
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
