# TFS-RUST 772 — Monster AI / Map: 1098 Mechanics Leak Audit

**Date:** 2026-06-27  
**Scope:** `crates/tfs-rust-core` monster AI, idle stimulus, pathfinding, map walkability, spawn placement, walk/todo integration  
**Related:** [`TFS-RUST_772_Real_Parity_Sim_Guide.md`](TFS-RUST_772_Real_Parity_Sim_Guide.md), [`TFS-RUST_772_RealMap_Parity_Trajectory.md`](TFS-RUST_772_RealMap_Parity_Trajectory.md)

---

## 1. Executive summary

| Verdict | Count | Meaning |
|---------|-------|---------|
| **Clean** | 30+ areas | Correctly gated on `beat_driven_loop` or `MechanicsProfile` |
| **Confirmed bug (772 path, 1098-like behavior)** | 0 open | §3.1–§3.3 fixed 2026-06-27 |
| **Potential 1098 leak (ungated TFS entry point)** | 1 open | Summon think stub on 772 idle (§13.1) |
| **Needs C++ confirmation** | 2 | Spawn leash on think; summon target sync vs `MasterFollow` |
| **Architecture OK (dual loop)** | — | 1098 and 772 use separate game-loop and think paths |

**Bottom line (pass 2):** After §3 fixes, no new **CRITICAL** 1098-style walk-poll leaks found. Real-map `kite_cyclops_one_real` Rust trace matches ref cadence (1× `todo_go`, `go_exec` @ 400/2000/4000). Remaining concern: **TFS `monster_think_summon_stub` still runs inside 772 `monster_idle_stimulus`** before native `MasterFollow` arms — instant `selectTarget` / `setFollowCreature` semantics that bypass idle `Strategy[]` (summons only).

---

## 2. Audit methodology

### 2.1 Primary gate (correct)

```rust
// game_world.rs — NOT clientVersion
beat_driven_loop = mechanics.profile.step_speed == StepSpeedModel::LinearGo;
```

All monster AI era selection should use **`self.beat_driven_loop`**, not `ProtocolVersion::V772` or `Codec::V772`. Wire codec checks in `walk/mod.rs` (`clear_todo_772`, `player_auto_walk`) are protocol concerns, not AI mechanics.

### 2.2 Secondary gate (profile-driven constants)

| Mechanism | Profile field | 772 default | 1098 default |
|-----------|---------------|-------------|--------------|
| Path search | `path_search` | `Reverse` | `Forward` |
| Path costs | `path_cost` | `TerrainWeighted` | `Fixed10/25` |
| Forward A* fallback | `path_forward_fallback` | `false` | `true` |
| Weakest target | `weakest_target_metric` | `CurrentHp` | `MaxHp` |
| Spawn placement | `spawn_placement` | `Classic772Bfs` | `TfsShuffle` |
| Follow repath without path | `follow_repath_without_path` | `true` | `false` |

### 2.3 Files reviewed

| Area | Files |
|------|-------|
| AI core | `monster_ai.rs`, `idle_stimulus.rs`, `monster_events.rs`, `monster_targets.rs`, `monster_distance_step.rs` |
| Think / loop | `creature_think.rs`, `game_world_tick.rs`, `game_loop.rs` |
| Walk / todo | `walk/mod.rs`, `walk/walk_timing.rs`, `creature_todo.rs` |
| Path / map | `pathfinding.rs`, `map/mod.rs`, `map/los.rs`, `map/grid.rs` |
| Spawn | `spawn_placement.rs`, `spawn_lifecycle.rs` |
| Combat / loot | `creature/monster_combat.rs`, `creature/monster_inventory.rs` |

### 2.4 Search patterns used

- All `beat_driven_loop` branches in monster/walk modules
- Callers of `monster_search_target`, `go_to_follow_creature`, `get_dance_step`, `get_distance_step`, `monster_on_think_target`, `monster_arm_event_walk`
- `MechanicsProfile` usage vs hardcoded TFS constants
- Map/pathfinder era selection via `uses_reverse_terrain_path`

---

## 3. Confirmed issues

**Status (2026-06-27):** All three issues fixed — gated on `beat_driven_loop` in `monster_events.rs` / `monster_ai.rs`. Real-map `kite_cyclops_one_real` Rust log: 1× `todo_go`, `go_exec` @ 400/2000/4000, `melee_hit` @ 4000.

### 3.1 CRITICAL — `close_flee_clear` inline idle repath (772 code, TFS-like behavior)

**Status:** Fixed — early return when in-flight `ToDoGo` or non-empty `walk_queue`; repath only via `monster_chase_needs_attacking_close_repath`.

**File:** `monster_events.rs` → `monster_close_chase_clear_pending_go_on_target_flee`

**Problem:** On every player kite step where cheb > 1 during close chase, Rust clears the todo queue and calls full `monster_idle_stimulus`. C++ 772 logs **one** initial `todo_go` and executes the path; Rust logs **one per walk tick**.

**Why it feels like 1098:** TFS `Monster::onThink` + `getNextStep` poll repath on every target move. 772 defers to idle segment drain + narrow `CreatureMoveStimulus` (`crmain.cc:888–961`).

**Evidence:** `kite_cyclops_one_real` — ref 1× `todo_go`, rust 6×; `go_exec` @ 400/2000/4000 vs 400/4000/5000.

**Fix:** Narrow to C++ `CreatureMoveStimulus` preconditions (locked attack todo, strike >200 ms away, cheb > 1). Do **not** full-idle-repath when a valid in-flight `ToDoGo` batch exists.

**Gate status:** Behind `beat_driven_loop` (772-only code path) — not a literal 1098 leak, but **1098-semantics on 772**.

---

### 3.2 MEDIUM — `selectTarget` on opponent move without follow

**Status:** Fixed — 772 uses `monster_schedule_chase_after_opponent_add`; TFS `monster_select_target` gated to 1098 only.

**File:** `monster_events.rs` ~187–197

```rust
// TFS Monster::onCreatureMove — monster.cpp ~287–289
if !is_summon && can_see_new && self.monster_is_opponent(...) && follow.is_none() {
    self.monster_ensure_opponent_listed(...);
    self.monster_select_target(monster_id, creature_id);  // NO beat_driven_loop gate
}
```

**Problem:** TFS instant target snap when an opponent moves into view and monster has no follow. On 772 this chains to `monster_set_follow_creature` → `request_idle_stimulus`, bypassing idle `Strategy[]` target pick (`crnonpl.cc:2420–2516`).

**772 reference:** Target acquisition from idle `Strategy[]` / `SetAttackDest` in idle drain (`crnonpl.cc:2784`), not TFS `selectTarget` on every move.

**Fix:** Gate with `if !self.beat_driven_loop { ... }` or replace with opponent-list update + deferred idle (match 772 appear/move semantics).

---

### 3.3 MEDIUM — Walk-to-spawn on opponent leave (TFS path)

**Status:** Fixed — `monster_maybe_walk_to_spawn` early-returns when `beat_driven_loop`.

**File:** `monster_targets.rs` → `monster_remove_creature_from_lists` → `monster_maybe_walk_to_spawn`

**Problem:** TFS `Monster::onCreatureLeave` → `walkToSpawn` (`monster.cpp` ~508). Ungated on `beat_driven_loop`. Uses:

- `get_creature_path_to` (profile-aware — OK)
- `creature_start_auto_walk` → `add_event_walk` (772 todo path, but not idle `Strategy[]` roam)

**Risk:** Monsters returning to spawn via TFS auto-walk when opponents leave viewport — may not exist or differ on 772.

**Fix:** Confirm 772 `crnonpl.cc` return-home behavior; gate or replace with 772 idle roam if absent.

---

## 4. Clean — correctly gated 1098 paths

These TFS 1098 mechanics **do not run** when `beat_driven_loop == true`:

| TFS path | Rust location | Gate |
|----------|---------------|------|
| `Monster::onThinkTarget` / `changeTargetSpeed` | `monster_on_think_target` | Early return if `beat_driven_loop` |
| `Monster::onThink` walk arm + searchTarget | `monster_native_on_think` else branch | `if !is_idle { if beat_driven { stall only } else { 1098 } }` |
| `Creature::goToFollowCreature` think repath | `creature_on_think` | `if !beat_driven_loop { go_to_follow... }` |
| `getDanceStep` / `staticAttackChance` poll | `monster_next_walk_step` | Early return on `beat_driven_loop` |
| Random roam in `getNextStep` | `monster_next_walk_step` | `if !beat_driven_loop { get_random_step }` |
| `monster_search_target` from think | `monster_native_on_think`, `idle_stimulus` | `!beat_driven_loop` branches only |
| `monster_try_acquire_chase_target` sync | `monster_targets.rs` | Redirects to `request_idle_stimulus` on 772 |
| `searchTarget` on appear | `monster_on_creature_appear_self` | `beat_driven → request_idle_stimulus` |
| `selectTarget` forced look | `monster_select_target` | Look update only if `!beat_driven_loop` |
| `creature_on_attacking` in ProcessCreatures | `process_creatures_772` | Skipped when `beat_driven_loop` |
| `process_walk_deadlines` Tokio poll | `game_loop.rs` dispatch | Skipped when `beat_driven_loop` |
| TFS forward A* relaxed retry | `monster_try_apply_chase_path` | Single FPP on 772; dual try on 1098 |
| `max_search_dist: 12` | `monster_path_search_params` | `0` on 772, `12` on 1098 |
| TFS `fullPathSearch` via `canUseAttack` | `monster_path_search_params` | Cheb band on 772; canUseAttack on 1098 |
| `monster_follow_repath_now` sync | `monster_ensure_follow_band` | `force_update_follow_path` on 772; sync repath on 1098 |
| `monster_reconcile_follow_position` | `monster_on_walk_complete` | Only when `!beat_driven_loop` |
| `get_distance_step` in follow | `go_to_follow_creature` | Redirects to `monster_idle_chase_repath` on 772 |
| 1098 corpse fallback | `monster_inventory.rs` | `!beat_driven_loop` gates |
| `schedule_walk_followup_deadline` poll | `check_creature_walk` | 772 monsters use `request_idle_stimulus`; else branch is 1098 |

---

## 5. Architecture — dual loops (correct)

| Concern | 1098 | 772 |
|---------|------|-----|
| Game loop | `run_game_loop_1098` → `on_tick` @ 50 ms | `run_game_loop_772` → `advance_beat_772` |
| Creature think | `check_creatures` staggered buckets | `process_creatures_772` ~1 Hz full sweep |
| Monster AI hub | `monster_native_on_think` + walk poll | `idle_stimulus` + todo queue drain |
| Combat | `creature_on_attacking` → (1098 body TBD) | `monster_do_attacking` via todo `Attack` |
| Walk timing | Tokio `next_walk_check` | `server_ms` + todo heap |

**Note:** `monster_do_attacking` returns immediately when `!beat_driven_loop` — 1098 monster melee is not implemented through this 772 combat port. That is a **772-only** path, not a leak into 772.

---

## 6. Map and pathfinding audit

### 6.1 Map layer — era-neutral (clean)

`map/mod.rs`, `map/grid.rs`, `map/los.rs` have **no** `beat_driven_loop` checks — correct. Walkability and LOS are shared; era differences live at call sites.

### 6.2 Pathfinding — profile-driven (clean)

| Check | Status |
|-------|--------|
| `uses_reverse_terrain_path(profile.path_cost, profile.path_search)` | 772 → reverse TShortway; 1098 → forward |
| `path_forward_fallback` | `false` on 772 (NOWAY, no TFS forward retry) |
| `get_path_matching_with_fill` + `fillmap_waypoints_at` | 772 FillMap uses `is_unpass_772`, terrain bank waypoints |
| Diagonal cost ×3 | Applied via `MechanicsProfile` in walk timing, not TFS 10/25 |
| `CHASE_PATH_MAX_STEPS` trim | 772 `ToDoGo` batch trim in `monster_try_apply_chase_path` |

### 6.3 Walk timing — profile-driven (clean)

`walk/walk_timing.rs`:

- 772: `LinearGo` → `linear_go_step_duration_ms`, NotifyGo quantizer
- 1098: `EraDefault` / retail log curve, 50 ms beat

No hardcoded 1098 speed formula on the 772 path.

### 6.4 Spawn placement — profile-driven (clean)

`spawn_placement.rs` matches on `mechanics.profile.spawn_placement`:

- 772: `Classic772Bfs` (`SearchSpawnField`)
- 1098: `TfsShuffle`

---

## 7. Idle stimulus — 772 hub (mostly clean)

`idle_stimulus.rs` is the 772 AI executor. Key 1098 exclusions:

| Behavior | 772 implementation | 1098 blocked |
|----------|---------------------|--------------|
| Target pick | `monster_idle_772_acquire_target` Strategy[] | `monster_search_target` in think path gated |
| Melee dance | `monster_idle_dance_step` (cardinal rand(0,4)) | `get_dance_step` not called |
| Chase repath | `monster_idle_chase_repath` + TShortway | `go_to_follow_creature` TFS body skipped |
| Flee | `search_flight_field` | TFS `getDistanceStep` flee in goToFollow skipped |
| Roam | idle `Roam` arm + `ToDoWait` | `get_random_step` in getNextStep skipped |
| Combat enqueue | `monster_enqueue_todo_attack_actions` | TFS `doAttacking` interval skipped |

**Exception:** `close_flee_clear` (see §3.1) fires from `monster_events`, not idle classify — bypasses idle cadence.

---

## 8. Codec vs mechanics — watch item

`walk/mod.rs` uses `matches!(self.codec, Codec::V772(_))` for:

- `clear_todo_772` before player move
- `first_only` auto-walk queue drain

This is **wire/protocol** behavior, not monster AI. Safe when `clientVersion` and `MechanicsProfile` stay aligned. If they ever diverge in config, player walk clearing could mismatch AI era — prefer `beat_driven_loop` for any future AI-adjacent walk logic.

---

## 9. Recommended fixes (priority order)

| P | Issue | Action | Test |
|---|-------|--------|------|
| ~~**P0**~~ | ~~`close_flee_clear` over-repath~~ | **Fixed** §3.1 | `kite_cyclops_one_real`: 1× `todo_go` |
| ~~**P1**~~ | ~~`selectTarget` on opponent move~~ | **Fixed** §3.2 | `test_772_opponent_move_defers_select_target` |
| ~~**P2**~~ | ~~Walk-to-spawn on leave~~ | **Fixed** §3.3 | `test_772_skips_walk_to_spawn_on_opponent_leave` |
| **P1** | Summon think stub on 772 idle (§13.1) | Gate `!beat_driven_loop` or use `MasterFollow` only | Unit: summon defers target sync |
| **P2** | Spawn leash on think (§13.2) | Confirm 772 C++; gate if absent | Scenario: monster beyond despawn radius |
| **P3** | Audit guard test | Add `test_772_no_1098_think_paths_fire` | CI regression |

---

## 10. Verification checklist

Run after any monster AI change on a **772** server (`clientVersion = 772`, `MechanicsProfile` LinearGo):

```bash
# Headless real-map — movement core
TFS_SIM_SEED=772 TFS_KITE_NO_WILD=1 \
  python3 scripts/run_kite_scenario.py --real-map \
  scripts/scenarios/kite_cyclops_one_real.scenario

# Inspect: todo_go count == 1, go_exec ticks 400/2000/4000
python3 scripts/summarize_chase_gaps.py \
  --ref log/chase_path_cip_realmap.log \
  --rust log/chase_path_rust_realmap.log \
  --monster cyclops --max-tick 5000
```

**Red flags (1098 leak or 1098-like behavior on 772):**

- [x] `branch` events with TFS roam/melee_dance arms on real-map kite (772 uses `todo_go` arms) — cyclops real-map OK post-§3.1
- [x] Multiple `todo_go` per harness walk tick during U-loop — fixed (1× on `kite_cyclops_one_real`)
- [x] `getDanceStep`-style diagonal dance from walk timer poll (not idle dance) — gated in `monster_next_walk_step`
- [x] `changeTargetSpeed` retarget mid-fight (no `monster_on_think_target` on 772) — gated
- [x] Forward A* path on terrain when reverse TShortway should run — profile-driven
- [x] Instant `selectTarget` snap on every opponent tile change — fixed §3.2
- [ ] TFS summon stub sync target before idle `MasterFollow` (§13.1 — summons only)

**Unit tests to run:**

```bash
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core \
  test_772_think_skips_creature_on_attacking \
  test_772_change_target_only_on_process_creatures \
  test_772_attacking_idle_tail_label_when_close_chase_skipped \
  uses_reverse_terrain_path_matches_772_profile \
  -- --nocapture
# P3: add test_772_no_1098_think_paths_fire once §3.1–§3.2 fixes land
```

---

## 11. Module scorecard

| Module | 1098 leak risk | Notes |
|--------|----------------|-------|
| `monster_ai.rs` | **Low** | Gates thorough; spawn leash ungated but likely era-shared |
| `idle_stimulus.rs` | **Low–Medium** | 772 hub; §13.1 summon stub on beat path |
| `monster_events.rs` | **Low** | §3.1–§3.2 fixed; `dist_follow_move` is intentional 772 dist-chase |
| `monster_targets.rs` | **Low** | §3.3 walk-to-spawn fixed |
| `monster_distance_step.rs` | **Low** | Only called from gated paths on 772 |
| `creature_think.rs` | **Low** | Dual think paths correct |
| `walk/mod.rs` | **Low** | Dual walk; codec checks are wire |
| `pathfinding.rs` | **Clean** | Profile-driven |
| `map/*` | **Clean** | Era-neutral |
| `spawn_placement.rs` | **Clean** | Profile enum |
| `game_loop.rs` | **Clean** | Separate 772/1098 loops |

---

## 13. Pass 2 audit (2026-06-27, post §3 fixes)

Re-scanned all `monster_*`, `idle_stimulus`, `creature_think`, `walk/mod`, `pathfinding`, spawn, and game-loop dispatch. Search patterns: ungated callers of `monster_search_target`, `go_to_follow_creature`, `monster_follow_repath_now`, `get_dance_step` / `get_random_step`, `monster_arm_event_walk`, `creature_on_attacking`, and `beat_driven_loop` branches in move/think handlers.

### 13.1 MEDIUM — TFS summon think stub on 772 idle

**File:** `idle_stimulus.rs` → `monster_idle_stimulus` ~942–943

```rust
if is_summon {
    self.monster_think_summon_stub(cid);  // no beat_driven_loop gate — runs on 772
}
```

**Problem:** `monster_idle_772_acquire_target` explicitly skips summons (`Strategy[]` — `crnonpl.cc:2420`), but the next block runs TFS `Monster::onThink` summon body (`monster_think_summon_stub` → instant `monster_select_target` / `monster_set_follow_creature` from master). Native 772 **`MasterFollow`** idle arms exist (`monster_idle_master_follow`, `test_772_classify_master_follow`) and run afterward in the same idle pass — duplicate / conflicting target sync.

**Risk:** Summons snap to master’s `attack_target` synchronously instead of deferring entirely to idle drain. Walk may still be 772-correct (`request_idle_stimulus` on `set_follow`), but **target pick semantics** are TFS-shaped.

**Fix (if C++ confirms):** Gate stub with `if !self.beat_driven_loop` or replace with master-band idle only (`MasterFollow` + opponent list from master).

**Test:** Unit: 772 summon with master in combat — assert no sync `selectTarget` before idle yield; follow acquired from idle classify only.

---

### 13.2 LOW — Spawn leash / despawn on think (ungated TFS path)

**File:** `monster_ai.rs` → `monster_native_on_think` → `monster_handle_out_of_spawn_range`

**Problem:** TFS `Monster::onThink` spawn-range check (`monster.cpp` ~760) runs on **both** eras. Uses TFS despawn/teleport (`remove_on_despawn`, POFF effect).

**Risk:** Unknown whether 772 `ProcessCreatures` uses identical leash semantics. Not a walk-poll leak; profile/config driven radii.

**Action:** Confirm against `crnonpl.cc` / 772 spawn lifecycle; gate only if absent.

---

### 13.3 LOW — Residual `walking_to_spawn` chain

**File:** `monster_ai.rs` → `monster_on_walk_complete` → `monster_walk_to_spawn`

**Status:** Entry gated (`monster_maybe_walk_to_spawn` returns on `beat_driven_loop`). Continuation chain still ungated if `walking_to_spawn` were ever set — currently only assigned inside `monster_walk_to_spawn` (1098 entry). Dead on 772 unless a future path sets the flag.

---

### 13.4 NOT a leak — `dist_follow_move`

**File:** `monster_events.rs` → `monster_on_follow_creature_moved`

Clears todo + calls `monster_idle_stimulus` on **dist-chase** (`target_distance > 1`) follow-target moves. Documented intentional 772 behavior (`Sim_Divergence_Report` D3; `hunter_chase` parity work). Differs from close-chase fix (§3.1) where C++ commits to initial `ToDoGo` batch without per-tile repath.

---

### 13.5 Pass 2 — confirmed clean (no new issues)

| Area | Result |
|------|--------|
| `go_to_follow_creature` / `monster_follow_repath_now` call sites | All gated: 1098 think repath, `set_follow`, `ensure_band`, `target_move` else branches |
| `monster_search_target` / `monster_on_think_target` | Only reachable when `!beat_driven_loop` (except inside gated helpers) |
| `monster_next_walk_step` dance/roam poll | `get_dance_step` / `get_random_step` skipped on `beat_driven_loop` |
| `check_creatures` / `process_walk_deadlines` | 1098 loop only; 772 uses `advance_beat_772` + `drain_todo_queue` |
| `creature_on_attacking` on 772 | Skipped in `process_creatures_772` when `beat_driven_loop` |
| `monster_start_chase_walk` | 772 → `idle_enqueue_go_and_start`; 1098 → `creature_start_chase_auto_walk` |
| `on_walk` empty-queue monster branch | 772 → `request_idle_stimulus`; 1098 → `schedule_walk_followup_deadline` |
| Path / spawn / map | Still profile-driven (unchanged from §6) |
| §3 red flags (post-fix) | `kite_cyclops_one_real` Rust: 1× `todo_go`, correct `go_exec` ticks |

---

### 13.6 Still open — regression guard (was P3)

Add `test_772_no_1098_think_paths_fire`: beat world, appear + kite fixture, assert no sync `searchTarget`, no `getDanceStep` from walk timer, single close-chase `todo_go` arm.

---

## 14. Related documents

| Doc | Use |
|-----|-----|
| [`TFS-RUST_772_Real_Parity_Sim_Guide.md`](TFS-RUST_772_Real_Parity_Sim_Guide.md) | Sim vs live; trace compare workflow |
| [`TFS-RUST_772_Real_Map_Kite_Sim_Plan.md`](TFS-RUST_772_Real_Map_Kite_Sim_Plan.md) | §2.2 sim vs live feel |
| `.cursor/rules/TFS-mechanics-profile.mdc` | Profile axis rules |
| `.cursor/rules/TFS-protocol-versioning.mdc` | No `*_772` in core APIs |
