# TFS-RUST 7.72 Chase Path Parity — Gap Analysis

**Date:** 2026-06-07 (updated 2026-06-08 — P0 terrain waypoints resolved)  
**Scope:** Live `chase_path.log` compare (CipSoft `tibia-game-master` vs TFS-RUST), static `TShortway` harness, chase debug instrumentation  
**Related:** [`TFS-RUST_772_Monster_AI_Dance_Chase_Parity.md`](TFS-RUST_772_Monster_AI_Dance_Chase_Parity.md), [`TFS-RUST_772_Pathfinding_Creature_AI_Analysis.md`](TFS-RUST_772_Pathfinding_Creature_AI_Analysis.md), [`TIBIA_GAME_MASTER_DEV.md`](TIBIA_GAME_MASTER_DEV.md)

---

## Executive summary

Both servers log chase pathing correctly (`branch`, `shortway`, `go_exec`; CipSoft also logs `todo_go`). The **reverse `TShortway` algorithm matches on synthetic grids** (5/5 exact, 0 hard mismatches). Live step-by-step parity **does not match yet** because of:

1. ~~**Wrong terrain waypoint costs** on the real map (`min_wp` 150 vs CipSoft 100–120)~~ — **✅ resolved** (see P0 terrain below)
2. **Noisy repath cadence** — 772 **todo execution is implemented**, but path refresh still fires far more often than CipSoft `ToDoGo(..., 3)` batching
3. **No controlled replay** (logs were separate sessions, different areas)

Next: **tighten repath triggers** (P1), then run a **controlled scenario** (same spawn, same player kite path, split log files) to verify waypoint parity in live logs.

---

## Evidence sources

| Source | What it tells us |
|--------|------------------|
| `log/chase_path.log` | 525 CipSoft + 488 Rust JSONL events (mixed runs) |
| `scripts/compare_chase_live_logs.py` | Pairwise diff tooling (needs split logs + same scenario) |
| `scripts/compare_chase_pathfinding.py` | Isolated `TShortway` on synthetic grids |

**Live log snapshot (2026-06-07):**

| | CipSoft | Rust |
|---|---------|------|
| Events | 525 | 488 |
| Monsters | dog, snake, wolf | snake, wolf |
| Coordinate footprint (x/y) | 32358–32423 / 32112–32214 | 32409–32423 / 32115–32130 |
| Exact `(start, dest)` shortway overlap | **0** | **0** |

---

## Confirmed gaps — change paths on the real map

### P0 — Terrain waypoint cost (`min_wp`) ✅

**Status:** Resolved 2026-06-08. Live log re-verify pending (needs controlled replay).

CipSoft `TShortway::FillMap` reads BANK tile **`WAYPOINTS`** (`cract.cc`). Rust pathfinding uses **`tile_ground_speed()`** → OTB `ITEM_ATTR_SPEED`.

**Was observed in live logs (2026-06-07, pre-fix):**

| Source | `min_wp` distribution |
|--------|------------------------|
| CipSoft | 100 ×4, 110 ×1, **120 ×84** |
| Rust | **120 ×98, 150 ×78** |

`min_wp` seeds every expand/heuristic term. Wrong costs → different paths even when start/dest match.

**Done:**

| Piece | Location |
|-------|----------|
| OTB `ITEM_ATTR_SPEED` patched from `objects.srv` `Waypoints` (75/75 walkable BANK) | `data/items/items.otb`, `patch-otb-waypoints`, `audit_cipsoft_waypoints` |
| `cipsoft_effective_waypoints` (0 → 150, not 1); leave-tile + diagonal ×3 | `pathfinding.rs` |
| `scan_cipsoft_min_waypoints` — FillMap min over walkable ±10 | `pathfinding.rs`, chase debug in `monster_ai.rs` |
| CipSoft viewport 10 / 441 cap (not TFS `maxSearchDist` 12 on 772) | `pathfinding.rs`, `monster_ai.rs` |
| Unwalkable tiles excluded from cost scan (no 150 on blocked) | `monster_ai.rs` `ground_cost` closure |

**Verify:** `cargo test -p tfs-rust-content --test audit_cipsoft_waypoints -- --nocapture` → 75/75 match.

---

### P0 — No controlled replay

Logs were **different sessions** in different map areas (e.g. CipSoft dog chase near `32368,32214`; Rust wolf/snake near `32416,32120`). Fuzzy pairing (same monster, dest within 3 tiles) found **123 pairs, 0% identical steps**.

**Fix direction:** Same monster spawn, same player route, split logs:

```bash
# CipSoft
export TIBIA_CHASE_PATH_DEBUG=1
scripts/tibia_game_online.sh start

# Rust
export TFS_CHASE_PATH_DEBUG=1
export TFS_CHASE_PATH_LOG=log/chase_path_rust.log
cargo run -p tfs-rust-server
```

Then compare with `scripts/compare_chase_live_logs.py` after normalizing monster names.

---

## Confirmed gaps — behavior / scheduling

### P1 — 772 todo queue: execution ✅, repath cadence ❌

**772 chase step execution is implemented** (not the old 1098 instant-walk path):

| Piece | Status | Rust location |
|-------|--------|---------------|
| `CreatureAction::Go` + todo heap | ✅ | `creature_todo.rs`, `walk/mod.rs` `process_creature_todo` |
| One step per `Go` → `on_walk` → chain next `Go` | ✅ | `idle_stimulus.rs` `execute_creature_todo_go`, `finish_creature_todo_execute` |
| Chase start via todo, not `add_event_walk` | ✅ | `monster_ai.rs` `monster_start_chase_walk` → `idle_enqueue_go_and_start` |
| Up to 3 dirs per repath | ✅ | `CHASE_PATH_MAX_STEPS = 3` in `pathfinding.rs` |
| 1098 still uses `creature_start_chase_auto_walk` | ✅ | gated on `!beat_driven_loop` |

CipSoft reference: `ToDoGo` → queue ≤3 steps → `ToDoStart` delay → `Go` → `NotifyGo` (`cract.cc`, `crnonpl.cc`).

**What still diverges:** *when* `go_to_follow_creature` runs (path refresh), not *how* individual steps execute.

#### Why live logs still look like “instant / noisy repath”

| Trigger | Behavior | Effect |
|---------|----------|--------|
| **`monster_ensure_follow_band`** (`idle_stimulus` line ~106) | If off-band while `walk_queue` non-empty → **clear queue** → **`monster_follow_repath_now`** (sync `go_to_follow_creature`) | Aborts in-flight 3-step batch before all steps run |
| **`followRepathWithoutPath = true`** (`772.lua`) | After each segment drain (`walk_queue` empty), idle repaths even when `has_follow_path` was set | Repath every 1 step, not every 3 |
| **`monster_on_follow_creature_moved`** | Clears `walk_queue`, sets `force_update_follow_path`, `request_idle_stimulus` | Player kiting interrupts batch on every target tile |
| **Chase debug `melee_chase_repath`** | Logged at **entry to every** `go_to_follow_creature` | Inflates branch count vs CipSoft `melee_chase` (207 vs 4 in sample) |

**Observed ratios (mixed live log, 2026-06-07):**

| Metric | CipSoft | Rust |
|--------|---------|------|
| `shortway` / `go_exec` | ~0.42 | ~1.71 |
| `branch` / `go_exec` (repath-ish / move) | ~0.02 | ~2.01 |
| `shortway` steps diagonal | ~3.0% | ~5.8% |

CipSoft batches up to 3 steps per `ToDoGo` before replanning via the next `IdleStimulus`. Rust queues 3 dirs but **player-move + `ensure_follow_band` + segment-drain repath** often flush the queue early.

**Fix direction (repath, not todo execution):**

1. On 772, **`monster_ensure_follow_band` must not call `monster_follow_repath_now`** — defer to idle flags like target-move does.
2. **Do not clear `walk_queue` in `ensure_follow_band` on 772** unless the path is truly stale (reuse hysteresis from `monster_on_follow_creature_moved`).
3. **Revisit `followRepathWithoutPath`** — true matches “repath on segment drain” but CipSoft drains after **3** queued steps, not 1.
4. **Chase debug:** log repath *reason* (`idle_drain`, `target_move`, `ensure_band`, `force_update`); wire `log_todo_go` at `idle_enqueue_go_and_start`; normalize branch name to `melee_chase`.

---

### P1 — Repath / branch cadence (log snapshot)

| | CipSoft | Rust |
|---|---------|------|
| `branch` events | 47 | 209 |
| `melee_chase*` | 4 (`melee_chase`) | 207 (`melee_chase_repath`) |
| `melee_dance` | 10 | 2 |
| `roam` | 33 | 0 |

- **CipSoft:** `TMonster::IdleStimulus` chooses arm once → `ToDoGo(..., max=3)` → todo runs batch → replan on drain (`crnonpl.cc`).
- **Rust:** todo runs steps correctly, but **`go_to_follow_creature` is invoked far more often** than CipSoft `ToDoGo` — see table above.

---

### P2 — Diagonal step rate (likely scheduling, not a separate move path)

From mixed live logs:

| | Total `go_exec` | Diagonal |
|---|-----------------|----------|
| CipSoft | 214 | 2 (**0.9%**) |
| Rust | 103 | 6 (**5.8%**) |

On **25 shared edges** `(monster, from→to)` present in both logs, **diag flag agreed 25/25** — geometry labels match where the same edge appeared.

**Root cause (not dance / `getDistanceStep` / greedy 8-dir):**

- All 6 Rust diagonal `go_exec` events matched a preceding `shortway` plan (SW corner cuts while kiting).
- Rust **`allow_diagonal: true`** in `TShortway` is correct; CipSoft also emits diagonal steps in `shortway` (~3%).
- Higher Rust rate comes from **replanning almost every tile** while the player moves diagonally — each fresh A* often picks one SW intercept step.
- **772 dance** uses `rand() % 5` cardinal only; **772 fallback** is 4-cardinal greedy, not 8-dir.

**Fix direction:** Re-measure after P1 repath-cadence fixes (P0 terrain ✅); diagonal rate should drop with CipSoft-style 3-step batching before replan.

---

### P2 — Roam not observed on Rust

CipSoft logged **33** `roam` branches; Rust sample had **none** (idle roam may not trigger or not be logged at the same layer).

**Fix direction:** Verify 772 idle roam path in `idle_stimulus` / `monster_next_walk_step`; add `roam` branch to Rust chase debug if implemented.

---

## Algorithm — mostly OK in isolation

### Core `TShortway` reverse A*

`python scripts/compare_chase_pathfinding.py`:

| Result | Count |
|--------|-------|
| Exact match | **5/5** |
| Tie (same cost, expand-order diff) | 3 |
| Hard mismatch | **0** |

The **graph search** is close; remaining live gaps are **AI shell (repath cadence)**, not the bare A* loop or terrain costs (P0 ✅).

---

### Expand-order ties on equal cost

CipSoft linked-list open set vs Rust `BinaryHeap` can pick different paths when multiple routes have **equal cost** (e.g. diagonal kite vs cardinal strip). Only matters when costs tie — `min_wp` / terrain costs are now aligned (P0 ✅).

---

## Logging / compare tooling gaps

These do not change gameplay but block 1:1 log diff:

| Gap | CipSoft | Rust |
|-----|---------|------|
| Monster names | `"a wolf"` | `"Wolf"` |
| Branch names | `melee_chase` | `melee_chase_repath` |
| `todo_go` events | yes | **no** (`log_todo_go` exists in `chase_debug.rs` but is not wired) |
| Default log file | `{LOGPATH}/chase_path.log` | `log/chase_path.log` (same path if env unset → **mixed file**) |

**Fix direction:** Always set `TFS_CHASE_PATH_LOG` separately; normalize names in compare script; wire `log_todo_go` at `idle_enqueue_go_and_start`.

---

## Priority roadmap

| Priority | Gap | Status / fix direction |
|----------|-----|------------------------|
| **P0** | `min_wp` / ground speed | **✅ Done** — patched `items.otb` + `pathfinding.rs` / `monster_ai.rs` (audit 75/75) |
| **P0** | Controlled repro | Same tile, monster, player path; split log files |
| **P1** | Repath cadence | Stop sync repath in `ensure_follow_band`; batch 3 steps before replan; revisit `followRepathWithoutPath` |
| **P1** | Chase debug | Wire `todo_go`, log repath reason, normalize `melee_chase` branch name |
| **P2** | Diagonal feel | Controlled test after P1 (P0 terrain ✅); gate diagonals or timing if still off |
| **P2** | Roam | Idle roam parity + debug branch |
| **P3** | Expand-order ties | Accept or mirror CipSoft linked-list tie-break |
| **P3** | Compare tooling | Name normalization, split-log workflow in docs |

---

## Event reference (debug schema)

Common JSONL fields: `src`, `evt`, `tick`, `id`, `name`, positions `from`/`to`/`start`/`dest`, `steps[]`, `min_wp`, `ok`, `diag`, `branch`, `must`, `max`, `cheb`.

| `evt` | CipSoft | Rust |
|-------|---------|------|
| `branch` | IdleStimulus arm | `go_to_follow_creature` entry (`melee_chase_repath`) / dance |
| `todo_go` | `ToDoGo` | not logged yet (todo execution uses `idle_enqueue_go_and_start`) |
| `shortway` | `TShortway::Calculate` | `get_creature_path_to_with_fpp` |
| `go_exec` | `TCreature::Go` | `internal_move_creature_step` |

Enable: see [Live monster pathing trace](TIBIA_GAME_MASTER_DEV.md#live-monster-pathing-trace-cipsoft--rust) in `TIBIA_GAME_MASTER_DEV.md`.

---

## Bottom line

| Question | Answer |
|----------|--------|
| Did both log? | **Yes** |
| Does live step-by-step match? | **No** — different runs, scheduling; costs fixed in code, not re-logged |
| Does isolated pathfinder match? | **Yes** — 5/5 synthetic scenarios |
| Is 772 todo execution missing? | **No** — `Go` + heap + step delay are in place |
| Terrain waypoints (`min_wp`) | **✅** — OTB + reverse A* aligned to CipSoft `WAYPOINTS` |
| Biggest remaining live suspect | **Noisy repath (not missing todo)** + **no controlled replay** |
