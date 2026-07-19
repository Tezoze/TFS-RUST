# Game-loop OBS baselines (Phase 0)

Capture after OBS-1 summaries are enabled. Run the server with:

```bash
RUST_LOG=tfs_obs=info,warn ./scripts/run_server.sh
```

Every **10 seconds** the game thread emits one structured line:

```text
target=tfs_obs message=game_obs_summary …
```

Paste p50/p95/p99 fields from a **60-second** window (six consecutive summaries, or one summary after a 60s scenario) into the tables below.

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

| Scenario | beat_lateness p50 | p95 | p99 | beat_wall p50 | p95 | p99 |
|---|---:|---:|---:|---:|---:|---:|
| Idle world | | | | | | |
| Dense spawn | | | | | | |
| Active chase | | | | | | |
| Spell-heavy fight | | | | | | |
| Corpse wave | | | | | | |
| Packet flood | | | | | | |

## Subsystem wall (µs)

| Scenario | creatures p50/p95/p99 | cron p50/p95/p99 | skills p50/p95/p99 | other p50/p95/p99 | todo p50/p95/p99 |
|---|---|---|---|---|---|
| Idle world | | | | | |
| Dense spawn | | | | | |
| Active chase | | | | | |
| Spell-heavy fight | | | | | |
| Corpse wave | | | | | |
| Packet flood | | | | | |

## ToDo / decay / path (counts + lateness)

| Scenario | todo_heap_max | todo_popped | todo_executed | todo_stale | todo_lateness p95 | decay_due | path_searches | path_failures | path_us p95 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Idle world | | | | | | | | | |
| Dense spawn | | | | | | | | | |
| Active chase | | | | | | | | | |
| Spell-heavy fight | | | | | | | | | |
| Corpse wave | | | | | | | | | |
| Packet flood | | | | | | | | | |

## Notes

- Command age is **game-thread visibility age** (pending deque / first receive), not wire ingress age.
- Writer age is not yet exposed; `output_queued_bytes_max` / `output_full` / `output_slow_shed` are.
- Do **not** start TODO-2 overload caps until these baselines + a production-shaped load-test show synchronized all-due drain is still the bottleneck.
- Audit: [`GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md`](GAME_LOOP_DECAY_IDLE_TODO_PERFORMANCE_AUDIT.md).
