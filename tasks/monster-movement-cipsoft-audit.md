# Monster movement vs CipSoft 772 — audit

**Date:** 2026-07-13 (pass 1–4 fixes applied)  
**Authority:** `reference/cipsoft-772/tibia-game-master` only (`crnonpl.cc`, `cract.cc`, `crcombat.cc`, `crmain.cc`, `main.cc`). TVP ignored.

---

## Pass 4 (2026-07-13) — fixed (P4-1..P4-4)

Verified-OK areas re-confirmed this pass: subsystem counters (1750/1500/1250/1000, reset 1000) vs `AdvanceGame`;
burst-beat coalescing vs `SigAlarmCounter`; `MoveCreatures` 1000 ms lag skip; heap drain due-check
(`cract.cc:785`); `NotifyGo` beat quantization + diagonal ×3 (`cract.cc:1513-1534` vs `walk_timing.rs`);
`CalculateDelay` `Wait` clamped to `EarliestWalkTime` (`cract.cc:905-915`); melee chase budget 3 /
dist-chase budget `Distance − 4` (`crnonpl.cc:2810`, `:2847`); dance `rand()%5` incl. no-step; roam
`rand()%4` ×10; queue orders `[Go,Wait,Attack]` (dist dance) and `[Go,Wait]` (roam); flee skips attack
tail; `Rotate` gated on `ATTACKING|PANIC`.

### P4-1 (HIGH) — Cornered flee freezes instead of roaming — **FIXED**

C++ `crnonpl.cc:2752-2759`: `SearchFlightField` failure has no else — falls into roam tail (`:2902`).
Rust: `monster_idle_prepare_and_enqueue_go` now re-executes `Roam` when `Flee` returns `Hold` (`DistFlee`
still maps to `QueuedWait` only).

### P4-2 (HIGH) — Master-follow band diverges at Manhattan 3 and ≤1 — **FIXED**

C++ `crnonpl.cc:2760-2777`: dist==2 Wait-only; dist==3 `ToDoWait` then `ToDoGo(max 3)`; dist≤1 falls
through to roam. Rust: `monster_master_follow_wait_only_band` (dist==2), `QueuedWaitThenGo` at dist==3,
`FallthroughRoam` at dist≤1; `idle_enqueue_wait_then_paced_go`.

### P4-3 (MEDIUM) — Fresh non-`Go` batch executes same-beat after idle stimulus — **FIXED**

C++ `cract.cc:789-793` breaks after `IdleStimulus`. Rust `process_creature_todo` now defers **all**
fresh batches when `next_wakeup` is unset or future, not only `Go`-fronted queues.

### P4-4 (VERIFY) — `flee_opening_melee_dance_done` (X3) — **REMOVED**

No decompile support (`IsFleeing()` checked first at `crnonpl.cc:2752`). Field + classify branch deleted.

### P4-5 (VERIFY) — `ATTACKING` promotion gate narrower than C++ — **DEFERRED**

C++ `crnonpl.cc:2779-2781`: `Skills[FIST] > 0 && State != PANIC` → `ATTACKING`, regardless of
`DistanceFighting`. Rust `monster_idle_maybe_enter_attacking` (`idle_stimulus.rs:1596-1598`) adds
`target_distance <= 1 || no ranged spell` — a dist fighter with melee skill never enters `Attacking`
via idle, changing its rotate/`ToDoAttack` tail. May be deliberate (TFS domain models ranged
auto-attacks as spell entries) — confirm against harness traces before changing.

### P4-6 (INFO) — Dist band is per-type `targetDistance`, decompile hardcodes 4

`crnonpl.cc:2837/:2845/:2863` use literal `4`; Rust uses `monster_effective_target_distance`
(monsters.xml). Current data pack only uses `targetdistance` 1 or 4, so outcomes match today — a pack
edit to 2/3/5 would diverge from the 772 reference.

---

## Short answer (post-fix)

**Scheduler feel bugs from pass 2–3 are fixed.** Dist mid-batch wipe removed; `Wait` uses absolute enqueue deadlines; `LockToDo` held for the whole batch until drain/`ToDoClear`.

| Area | Verdict |
|---|---|
| `advance_beat` → drain | **OK** |
| Heap due / `ToDoStart` +1 clamp | **OK** |
| Go pacing / `NotifyGo` | **OK** |
| Multi-step chase (`1×Go` + `walk_queue`) | **OK** (model differs from N×`TDGo`; lock + no dist wipe keep it coherent) |
| `LockToDo` batch | **Fixed** — set on `ToDoStart`, clear on drain/`ToDoClear` |
| Dist target-move repath | **Fixed** — no mid-batch clear (CLOSE-only combat path unchanged) |
| `Wait` absolute deadline | **Fixed** — `CreatureAction::Wait { deadline_ms }` |

---

## Fixes applied (2026-07-13)

1. **`monster_on_follow_creature_moved`** — no-op. Dist kite no longer clears todo/`walk_queue`/`next_wakeup` or idle-repaths inline (`crmain.cc:920` CLOSE + head `TDAttack` only; close path stays in `monster_combat_creature_move_stimulus`).
2. **`CreatureAction::Wait { deadline_ms }`** — absolute at enqueue (`cract.cc:1033`); execute uses `max(deadline, EarliestWalkTime)` (`cract.cc:905-915`).
3. **Batch `LockToDo`** — `todo_start_from_action` / immediate wakeup set `locked`; execute no longer unlocks per action; `creature_todo_release_lock_if_drained` clears when todo + `walk_queue` empty.

### Tests added/updated

- `dist_monster_keeps_walk_queue_when_follow_target_moves`
- `test_772_dist_target_flee_does_not_preempt_goal_wait`
- `test_monster_wait_deadline_is_absolute_at_enqueue`
- `test_monster_lock_todo_held_between_go_steps`
- Absolute deadline asserts in catch / hard-block tests

```bash
rtk cargo test -p tfs-rust-core --lib
```

---

## Historical notes (pass 1–3)

Pass 1: path/idle/beat structure largely matched CipSoft (MaxSteps=3, TShortway cardinals, dance/roam).  
Pass 2–3: found dist interrupt, relative Wait, weak LockToDo — see git history of this file / conversation transcript if needed.

## Code map

| Concern | Rust | CipSoft |
|---|---|---|
| Beat drain | `game_world_tick.rs` `advance_beat` | `MoveCreatures` |
| Heap / process | `walk/mod.rs` `drain_todo_queue` / `process_creature_todo` | `ToDoQueue` + `Execute` |
| Finish / re-arm | `idle_stimulus.rs` `finish_creature_todo_execute` | `Execute` loop |
| Dist kite | `monster_events.rs` (no-op) | *(no non-CLOSE equivalent)* |
| Close kite | `monster_combat_creature_move_stimulus` | `crmain.cc:920` |
| Wait absolute | `creature_todo.rs` `enqueue_creature_wait` | `ToDoWait` |
| LockToDo | `todo_start_from_action` + `creature_todo_release_lock_if_drained` | `ToDoStart` / `ToDoClear` |

---

## Terrain-weighted / TShortway pass (2026-07-13)

**Verdict:** core FillMap + Expand + NotifyGo **match** 772 `cract.cc`. TW-1..TW-3 **fixed**.

| ID | Severity | Issue | Status |
|---|---|---|---|
| TW-1 | MED | Dist truncate used `stop_at_cheb=target_distance` | **Fixed** — `truncate_tshortway_go_queue` stops at cheb≤1; band via MaxSteps |
| TW-2 | LOW | Empty viewport MinWaypoints → 150 | **Fixed** — keep FillMap seed 1000 |
| TW-3 | LOW | Dist budget `.max(1)` vs C++ can be 0 | **Fixed** — `.max(0)`; empty truncate after reachable path = success |
| TW-4 | INFO | Passable OTB `0` → FillMap/NotifyGo default **150**; Unpass stays `-1` via `blockSolid` | **Aligned** |
| TW-5 | INFO | Flee = `SearchFlightField`, not multi-step TShortway | OK |
| TW-6 | HIGH | Unpass mountains (XML "rock soil" 4411+) missing OTB `FLAG_BLOCK_SOLID` | **Fixed** — offline flag patch |
