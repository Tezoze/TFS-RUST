# Game-loop OBS baselines (Phase 0)

Capture after OBS-1 summaries are enabled. **Opt-in only** — default binary filter is `tfs_obs=off` (no 10s `game_obs_summary`).

```bash
# Baseline capture:
RUST_LOG=info,tfs_obs=info ./scripts/run_server.sh 2>&1 | tee /tmp/tfs_obs.log

# Day-to-day: omit RUST_LOG (defaults mute tfs_obs), or:
RUST_LOG=warn ./scripts/run_server.sh
```

Every **10 seconds** (while `tfs_obs=info`) the game thread emits one structured line:

```text
target=tfs_obs message=game_obs_summary …
```

Paste p50/p95/p99 fields from a **60-second** window (six consecutive summaries, or one summary after a 60s scenario) into the tables below.

Day-to-day lag signal without obs spam: `WARN tfs_rust_core::game_world_tick: 772 beat advance timing` (fires only when a beat is actually slow).

## Scenarios

| Scenario | How to provoke |
|---|---|
| Idle world | Server up, no players / no combat |
| Dense spawn | Stand on / near a crowded monster floor |
| Active chase | Pull monsters and kite |
| Spell-heavy fight | Fight casters / multi-spell types |
| Corpse wave | Mass-kill leaving decaying corpses |
| Packet flood | Rapid walk / auto-walk / UI spam from client |

## Beat lateness + wall (ms)

Captured **2026-07-19** live (`/tmp/tfs_obs.log` + earlier session). Values are approximate from 10s `game_obs_summary` windows.

| Scenario | beat_lateness p50 | p95 | p99 | beat_wall p50 | p95 | p99 |
|---|---:|---:|---:|---:|---:|---:|
| Idle world | 0 | 0 | 0 | 0 | 0 | 2 |
| Dense spawn | *(optional)* | | | | | |
| Active chase (~20 monsters lure/kite) | 0 | 0 | 0 | 4–8 | 16 | 16–32 |
| Spell-heavy fight | *(optional)* | | | | | |
| Corpse wave (stage due windows) | 0 | 0 | 0 | 0–8 | 8 | 8–16 |
| Packet flood | *(optional)* | | | | | |

## Subsystem wall (µs)

| Scenario | creatures p50/p95/p99 | cron p50/p95/p99 | skills p50/p95/p99 | other p50/p95/p99 | todo p50/p95/p99 |
|---|---|---|---|---|---|
| Idle world | 0 / 0 / ~1k | 0 / 0 / 4 | 0 / 0 / ~2k | 0 / 0 / 512 | 0 / 0 / 0 |
| Dense spawn | | | | | |
| Active chase | 0 / 0 / ~1k | 0 / 0 / 4 | 0 / 0 / ~2k | 0 / 0 / 512 | ~4–8k / ~16k / ~16–32k |
| Spell-heavy fight | | | | | |
| Corpse wave (due windows) | 0 / 0 / ~1k | 0 / ~4–128 / **≤256** | 0 / 0 / ~2k | 0 / 0 / 512 | low (not the hot path) |
| Packet flood | | | | | |

## ToDo / decay / path (counts + lateness)

| Scenario | todo_heap_max | todo_popped | todo_executed | todo_stale | todo_lateness p95 | decay_due | path_searches | path_failures | path_us p95 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Idle world | 0–1 | ~20 | ~20 | 0 | 64 | 0 | 0 | 0 | 0 |
| Dense spawn | | | | | | | | | |
| Active chase | ~30–38 | ~400–550/10s | ~400–500 | ~25–45 | 64 | 0–15 live | ~100–150/10s | 0–46 | ~2k |
| Spell-heavy fight | | | | | | | | | |
| Corpse wave | low | low | low | 0 | — | **1–11**/10s (23 due windows; Σ110) | 0 | 0 | 0 |
| Packet flood | | | | | | | | | |

## Live findings (2026-07-19)

- **Decay:** Corpses transformed in-game. OBS: `decay_heap_max == decay_live_max` always; due bursts up to 11/10s; cron ≤256µs p99 during due — **not** a performance problem.
- **Chase:** ~20-monster lure/kite kept `beat_lateness` at 0 and beat wall in tens of ms (when not on the container bug).
- **MAP-walk bug (fixed):** Walking with an **open ground corpse/container** caused 350ms–1.1s `todo_us` and `MoveCreatures` skips via per-step `find_item_position`. Fixed with O(1) `script_item_position`; verified fixed after rebuild.
- `output_full` / `output_slow_shed` stayed 0 in these captures.
- **TODO-2:** not indicated by this data.

## Notes

- Command age is **game-thread visibility age** (pending deque / first receive), not wire ingress age.
- Writer age is not yet exposed; `output_queued_bytes_max` / `output_full` / `output_slow_shed` are.
- Corpse `duration` in `items.xml` is seconds (~1200s/stage for dead rat) — wait for stages; blood/poison fields are much shorter.
- Do **not** start TODO-2 overload caps until a denser load-test still shows synchronized all-due ToDo as the bottleneck.
- Audit: [`GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md`](GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md).
