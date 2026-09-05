# Game-loop OBS baselines (Phase 0)

Capture after OBS-1 summaries are enabled. **Opt-in only** — default binary filter is `tfs_obs=off` (no 10s `game_obs_summary`).

> **Release builds only.** `scripts/run_server.sh` runs `cargo run` with no `--release`
> (`run_server.sh:33`), so anything captured through it is an unoptimized build and is **not**
> a performance baseline. Use `scripts/capture_obs_baseline.sh`, which forces `--release`,
> enables `tfs_obs`, and stamps the log with build/commit/CPU.

```bash
# Baseline capture (release, one log per scenario):
./scripts/capture_obs_baseline.sh dense_spawn
# …play the scenario ≥90s (30s warmup + 60s measured), then Ctrl-C…
scripts/parse_obs_log.py /tmp/tfs_obs/dense_spawn.log --scenario "Dense spawn" --skip 3

# Day-to-day: plain ./scripts/run_server.sh (defaults mute tfs_obs).
```

Every **10 seconds** (while `tfs_obs=info`) the game thread emits one structured line:

```text
target=tfs_obs message=game_obs_summary …
```

`parse_obs_log.py` turns those lines into the table rows below; `--skip 3` discards the 30s warmup.

**Reading the numbers.** `obs.rs` uses a power-of-two `FixedHistogram` (`obs.rs:16-20`), so every
percentile is a bucket *upper edge* — 0, 1, 2, 4, 8, 16, 32, … A reported `16` means "in (8, 16]",
not "16". Percentiles from separate windows cannot be averaged, so where windows disagree the
tables show the observed range (`16–32`). Good enough for triage; not precise enough to publish.

The beat period is **50 ms** (`formulas.rs:564`), which is the budget `beat_wall` is spending against.

Day-to-day lag signal without obs spam: `WARN tfs_rust_core::game_world_tick: 772 beat advance timing` (fires only when a beat is actually slow).

## Scenarios

| Scenario | How to provoke | Capture |
|---|---|---|
| Idle world | Server up, no players / no combat | `./scripts/capture_obs_baseline.sh idle_world` |
| Dense spawn | Stand on / near a crowded monster floor | `./scripts/capture_obs_baseline.sh dense_spawn` |
| Active chase | Pull monsters and kite | `./scripts/capture_obs_baseline.sh active_chase` |
| Spell-heavy fight | Fight casters / multi-spell types | `./scripts/capture_obs_baseline.sh spell_heavy` |
| Corpse wave | Mass-kill leaving decaying corpses | `./scripts/capture_obs_baseline.sh corpse_wave` |
| Packet flood | Rapid walk / auto-walk / UI spam from client | `./scripts/capture_obs_baseline.sh packet_flood` |

Each needs ≥90s: the first 30s is warmup (spawn settle, page cache, allocator steady state),
discarded via `--skip 3`. Record roughly how many monsters were engaged — creature count is the
independent variable and the row is uninterpretable without it.

## Beat lateness + wall (ms)

Captured **2026-07-19** live (`/tmp/tfs_obs.log` + earlier session). Values are approximate from 10s `game_obs_summary` windows.

> ⚠️ **These rows are debug-build numbers** — captured via `run_server.sh`, which has never passed
> `--release`. Treat them as relative shape only; the absolute timings are meaningless as a
> performance baseline, and the empty rows should be filled from release captures instead.
> Re-capture the filled rows too, so the whole table is one build profile.

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
- Do **not** start TODO-2 overload caps until a denser load-test still shows synchronized all-due ToDo as the bottleneck. That decision is blocked on release-build numbers — the debug capture above cannot settle it either way.
- Benchmark scope beyond these live captures (microbenches, `sim_harness` scaling sweep, wire loadgen, TVP A/B) is planned in [`PERF_BENCHMARK_PLAN.md`](PERF_BENCHMARK_PLAN.md), tiered cheapest-first.
- Audit: [`GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md`](GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md).
