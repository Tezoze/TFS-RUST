# Beat Loop, Decay, Idle Stimulus, and ToDo Performance Audit

**Date:** 2026-07-19  
**Updated:** 2026-07-19 (post-landing regressions + next steps)  
**Primary comparison:** 772 mechanics reference (`tibia-game-master/src/`)  
**Scope:** static source audit of the single game thread. This is not a sampled CPU profile; suspected costs are ranked from code shape, known cardinalities, and existing live timing notes in `tasks/lessons.md` §205–§216.  
**Implementation checklist:** `tasks/todo.md`  
**Landed commit (this remediation):** `68b7c93`

## Status (2026-07-19)

Phases **1–4 code fixes are landed** (GL-1…GL-4, DEC-1…DEC-4, IDLE-1…IDLE-3 thin, TODO-1, OBS-1 thin), plus **post-landing session/decay/path regressions** (logout TCP close, LocalSet-safe saves, GameLaneFull shed, TShortway `expand_next`, `stop_decay` / look-save clock units, todo execute guard re-arm).

**OBS-1 full histograms are landed** (`crates/tfs-rust-core/src/obs.rs`, periodic `target=tfs_obs` summary every 10s). Live **60s baselines** and manual smoke remain operator work — see [`GAME_LOOP_OBS_BASELINES.md`](GAME_LOOP_OBS_BASELINES.md).

**Not done (next work):** manual smoke retests, Phase 0 live baseline capture, IDLE-3 sector order, optional GL-4 indexes, full-scale required tests, beat-startup parity note, load-test, and **TODO-2** (only if still needed after load-test).

Jump to [Next steps](#next-steps).

## Executive summary

The global ToDo heap and the basic 772 beat/counter formulas are not the principal performance problem. The largest lag risks are work around them:

1. **~~The game loop awaits player-load database I/O.~~** **Fixed (GL-1):** Tokio load → `PlayerLoaded` / `PlayerLoadFailed`; game thread applies only.
2. **~~Unbounded command channel + exhaustive drain before beats.~~** **Fixed (GL-2):** dual-lane `GameCmdTx` (bounded game / control) + per-turn command budget before servicing ready beats.
3. **~~Decay container/equip amplification.~~** **Fixed (DEC-1):** parent-cylinder equip path; batched empty; iterative destroy.
4. **~~Lazy decay cancellation.~~** **Fixed (DEC-2):** indexed min-heap cancel/reschedule (`CronDelete` shape).
5. **~~Idle spell rebuild + per-search path scratch.~~** **Fixed (IDLE-1 / IDLE-2):** merged spells at spawn; `TShortwayScratch` on `GameWorld`.
6. **~~Recursive ToDo Execute.~~** **Fixed (TODO-1):** explicit loop + iteration guard.
7. **Synchronized due work is unbounded by design.** Still correct — do **not** add ToDo/cron caps (TODO-2) until load-test of the optimized path.

Also landed: **GL-3** (bounded outbound + game-thread sink map), **DEC-3** (`DecayClockModel` / round clock under lag), **DEC-4** / **GL-4** / **IDLE-3** / **OBS-1** thin follow-ups.

## Reference behavior that must remain intact

| Area | 772 behavior | Rust status |
|---|---|---|
| Beat accumulation | `LaunchGame` consumes all pending alarm beats and calls `AdvanceGame(NumBeats * Beat)` | `MissedTickBehavior::Burst` plus coalesced `advance_beat` matches the shape (`game_loop.rs:647-729`) |
| Staggered systems | Creature/Cron/Skill/Other thresholds are 1750/1500/1250/1000 ms and subtract 1000 once | Correct (`subsystem_counters.rs:23-61`; `main.cc:318-350`) |
| Severe-lag movement | `Delay >= 1000` skips `MoveCreatures` | Correct for movement (`game_world_tick.rs:73-91`; `main.cc:440-453`) |
| ToDo due drain | Pop every entry whose key is `<= ServerMilliseconds` | Correct (`walk/mod.rs:386-429`; `crmain.cc:1142-1157`) |
| ToDo tie order | Structural 1-indexed heap order; no FIFO secondary key | Correct and intentionally custom (`todo_queue.rs:1-121`; `containers.hh:150-227`) |
| ToDo execution | `Execute` is a `while (true)` chain until delayed, stopped, dead, or drained | **Fixed:** explicit loop (`idle_stimulus.rs` `run_monster_todo_execute`; `cract.cc:783-898`) |
| Idle stimulus | Runs when a creature's ToDo list drains; inactive wild monsters may sleep without rearming | Correct overall (`idle_stimulus.rs`; `crnonpl.cc:2345-2939`) |
| Cron due drain | `ProcessCronSystem` repeatedly processes `CronCheck()` until no due object remains | Correct all-due shape (`decay.rs`; `operate.cc:2752-2783`) |
| Cron cancellation | Reference hash/index links remove a live heap entry with `CronDelete` | **Fixed:** indexed heap remove/update (`decay.rs`; `map.cc` `CronDelete`) |
| Container expiry | `Empty` runs before `Change` | Correct outcome; **apply path optimized (DEC-1)** |

## Prioritized findings

| ID | Priority | Hot path | Status |
|---|---:|---|---|
| GL-1 | **P0 / Critical** | Login dispatch | **Done** — async load / game-thread apply |
| GL-2 | **P0 / Critical** | Command ingress | **Done** — dual-lane + command budget |
| DEC-1 | **P0 / Critical** | Container/equipment expiry | **Done** — parent equip + batch empty |
| DEC-2 | **P1 / High** | Decay heap | **Done** — indexed cancel/reschedule |
| IDLE-1 | **P1 / High** | Spell-capable idle passes | **Done** — prebuilt spells / Arc talk |
| IDLE-2 | **P1 / High** | Chase repath | **Done** — `TShortwayScratch` reuse |
| TODO-1 | **P1 / High** | Zero-delay action chains | **Done** — iterative Execute |
| GL-3 | **P1 / High** | Outbound delivery | **Done** — bounded outbound + sink map |
| DEC-3 | **P1 / High** | Decay clock | **Done** — `DecayClockModel` / round clock |
| TODO-2 | **P2 / Medium** | Due ToDo drain | **Deferred** — only after load-test |
| IDLE-3 | **P2 / Medium** | Target/move searches | **Partial** — scratch reuse; sector-order/dedup still open |
| GL-4 | **P2 / Medium** | Periodic systems | **Partial** — scratch Vecs; active indexes still open |
| OBS-1 | **P1 / High** | Diagnostics | **Done (code)** — windowed histograms + `tfs_obs` summary; live baselines open |
| DEC-4 | **P2 / Medium** | `can_decay` / destroy | **Done** — cached depot flag; iterative destroy (via DEC-1) |

---

## Beat-driven game loop

### GL-1 — Player login performs database I/O while holding the game loop

`dispatch_command` awaits `handle_player_login`, which awaits `login_player`, which awaits `PlayerStore::load_player_full`:

- `game_loop.rs:176-195`
- `game_loop.rs:583-598`
- `login.rs:269-280`

The `GameWorld` is unavailable for the full query. A slow DB query, pool wait, network pause, or database outage freezes:

- beat advancement;
- ToDo and idle stimulus;
- decay and skills;
- all other players' commands;
- outgoing packet flushes.

This violates the repository's game-thread/I/O boundary even though it is memory-safe.

**Required change**

1. Split login into an async load phase and a game-thread apply phase.
2. Run `load_player_full` in a Tokio task using owned name/config/DB handles.
3. Send a `PlayerLoaded`/`PlayerLoadFailed` command containing owned data back to the game thread.
4. Validate that the connection is still current before inserting the player.
5. Bound concurrent login loads and reject or queue excess attempts outside the simulation thread.

This changes no successful-login outcome; it only prevents unrelated simulation from waiting on I/O.

### GL-2 — Biased, exhaustive draining can starve beats and grow memory without limit

The main channel is `mpsc::unbounded_channel` (`run_server.rs:167-173`). Network readers and Lua timers send into it (`protocol_game.rs:115-168`; `scheduler.rs:52-67`). The game loop uses a biased select with commands first, then drains `try_recv()` until the channel is empty (`game_loop.rs:683-729`).

Under sustained ingress, the channel may never become empty. The beat arm can remain ready but unselected indefinitely. Consequences:

- simulation stalls while packet parsing continues;
- the queue grows without backpressure;
- timer callbacks and player packets compete in one FIFO;
- missed beats later coalesce, causing a large synchronized subsystem pass;
- if the coalesced delay reaches 1000 ms, movement is skipped exactly when backlog is highest.

The 772 loop receives network data before an alarm beat when both signals are pending (`main.cc:484-497`), but that does not justify unlimited application-level buffering or permanent beat starvation.

**Required change**

- Replace the command channel with a bounded channel sized from a documented overload policy.
- Process a bounded command slice per loop turn (count and/or elapsed CPU budget), then explicitly service a ready beat.
- Reserve capacity or a separate lane for disconnect/shutdown/login-completion control messages.
- Apply per-connection packet-rate limits before the shared game queue.
- Preserve FIFO ordering within each connection; add differential tests for normal, non-overloaded ordering.

A command budget changes only overload behavior. Treat that as a deliberate production-safety policy, not as a hidden mechanics change.

### GL-3 — Output flush can block and slow clients have unbounded queues

`OutRegistry` is an `Arc<Mutex<HashMap<..., UnboundedSender<...>>>>` (`tfs-rust-net/src/server.rs:19-20`). Every beat takes the blocking mutex on the game thread and sends into unbounded per-connection channels (`game_loop.rs:95-103`; `server.rs:323-330`).

The critical section is short today, but a contended `std::sync::Mutex` can park the only simulation thread. More importantly, a client whose socket writer is slow can accumulate unlimited encoded batches.

**Required change**

- Make registry lookup lock-free from the game thread by keeping a game-thread-owned `ConnId -> Sender` map, updated through connection commands.
- Use bounded per-connection output channels with byte accounting.
- Disconnect or shed a slow client after a documented byte/time threshold.
- Expose queued bytes, not only queued batch count.

### GL-4 — Periodic scans align with due-work bursts

`process_creatures` scans the full creature SlotMap and creates three vectors (`creature_think.rs:37-71`). `process_skills` creates another filtered ID vector (`process_skills.rs:18-45`). These are reference-shaped full passes, but their cost lands in the same `advance_beat` call as cron, other systems, and the ToDo drain (`game_world_tick.rs:41-92`).

The vectors are not individually critical; the alignment is. Do not add an extra player-count scan merely to guess capacities—it doubles traversal. Better options are:

- maintain game-thread-owned active indexes for players, conditioned creatures, and dead-pending creatures;
- reuse scratch `Vec<CreatureId>` buffers stored on `GameWorld` if indexes are premature;
- preserve SlotMap IDs and validate existence before mutation.

### Beat timer startup note

`tokio::time::interval` has an immediately ready first tick (`game_loop.rs:678-680`), so logical time advances once immediately after startup. The reference waits for an alarm signal before `AdvanceGame` (`main.cc:484-497`). This is low performance risk but should be covered by a startup timing parity test; use `interval_at(now + beat, beat)` if the immediate advance is not intended.

---

## Decay system

### DEC-1 — Expiry application has severe avoidable amplification

The deadline heap is efficient when entries merely pop. Applying expiry is not.

#### Equipment lookup is `O(expired × players × slots)`

`process_decay_expiry` calls `find_equipment_owner` for every expired item (`decay_apply.rs:427-438`). That helper scans every creature and every player's equipment slots (`player/inventory/equip_abilities.rs:430-441`), even though each `Item` now has an O(1) parent cylinder.

**Fix:** resolve `item.parent`/`resolve_item_parent_cylinder` once. Only run equip removal when the parent is `Cylinder::Inventory`; carry the player and slot directly.

#### Container emptying repeats expensive work per child

For each excess child, `empty_container_for_expire`:

1. reads the first item and linearly looks it up again;
2. removes from the front of the container sequence;
3. refreshes the entire ancestor chain;
4. recomputes recursive weight and recursive item count;
5. notifies container viewers;
6. scans creatures to find carrying players through container trees.

Relevant paths:

- expiry loop: `decay_apply.rs:293-359`;
- recursive destroy and viewer clones: `decay_apply.rs:369-399`;
- full derived recomputation: `container_ops.rs:80-109`;
- owner scan and UI notification: `container_ui.rs:264-275, 397-435`.

For a container with `n` children, front removal and repeated aggregate recomputation are at least quadratic in common representations. Nested containers and multiple viewers add tree traversal and packet amplification. A synchronized corpse-expiry wave can therefore dominate the cron phase. Existing project notes record a live `cron_us` near four seconds before earlier parent/heap fixes; this path remains capable of similar spikes.

**Required change**

- Add a batch detach/empty primitive specialized for expiry.
- Determine the exact ordered child list once.
- Update parent links and decay entries in one pass.
- Maintain weight/item-count deltas incrementally up the parent chain instead of recursively recounting after every child.
- Preserve per-child protocol notifications and move/delete order where observable, but compute owner/viewer sets once.
- Replace recursive tree destruction with an explicit `Vec<ItemId>` stack.
- Add a direct fast path for `remainder == 0` corpse deletion that avoids repeated front shifts.

Do not merely cap expired items per cron tick: `ProcessCronSystem` processes all due objects. Optimize the application path first.

### DEC-2 — Lazy cancellation makes heap size proportional to historical churn

`DecayManager` combines a live `HashMap` with a `BinaryHeap`; cancel removes only the map entry and leaves the heap key (`decay.rs:47-75`). Rescheduling also leaves the previous key. Stale keys are removed only after their old deadline reaches the heap head (`decay.rs:84-101`).

This is safe functionally, but a frequently transformed or moved long-duration item can leave keys resident for minutes or hours. Costs:

- retained memory grows with schedule history;
- every live push/pop pays `O(log historical_heap_len)`;
- a mass old deadline causes a stale-pop burst;
- no current metric reveals `heap_len / live_entries`.

The 772 cron structure maintains object-to-position links and uses `CronDelete`, so cancellation/reschedule removes the live heap entry immediately (`map.cc:209-257, 286-325`).

**Required change**

Implement an indexed min-heap:

- `Vec<DecayHeapKey>` plus `HashMap<ItemId, heap_index>`;
- update indices on swaps;
- remove/update by `ItemId` in `O(log n)`;
- preserve deterministic `(deadline, ItemId)` ordering;
- keep `remaining_ms` in the live entry map or the indexed node.

A periodic heap rebuild is an acceptable short-term guard, but an indexed heap matches the reference behavior and permanently bounds heap size to live entries.

### DEC-3 — Decay uses the movement clock and freezes under the lag guard

Decay deadlines use `server_ms` (`decay_apply.rs:127-142`) and cron checks it before movement advances (`game_world_tick.rs:51-57, 73-91`). When a coalesced `delay_ms >= 1000`, the Rust loop still runs cron but does not advance `server_ms`; decay therefore does not progress. The reference cron is keyed by `RoundNr`, while only `MoveCreatures`/`ServerMilliseconds` is skipped (`map.cc:209-230, 259-281`; `main.cc:337-350, 445-452`).

This is an observable 772 divergence and can bunch expiries after lag recovery.

**Required change**

- Separate decay time from the movement clock.
- For 772, model deadlines from the round-based cron clock (`RoundNr + duration_seconds`).
- Keep the TFS duration/decay domain and persistence format shared; if 1098 needs millisecond timing, select the clock policy through `MechanicsProfile`, not a version check in decay logic.
- Test severe-lag advancement explicitly: movement may pause, expiry time must follow the active mechanics clock.

### DEC-4 — Smaller decay costs

- `can_decay` reparses `itemsDecayInsideDepots` for each call and may walk all container ancestors (`decay_apply.rs:30-87`). Cache the boolean at startup; keep the ancestor walk unless a depot-depth/root cache is measured necessary.
- `destroy_item_tree` is recursive (`decay_apply.rs:369-388`). Deep or malformed container nesting can overflow the game-thread stack; use iterative post-order traversal.
- `tick` returns a newly created vector (`decay.rs:85-101`). Empty vectors do not allocate, but large bursts do. Reuse an expiry scratch vector after the larger issues are fixed.

---

## Idle stimulus

### IDLE-1 — Immutable monster combat data is rebuilt on every casting pass

`monster_idle_try_casting` performs all of the following per pass (`idle_stimulus.rs:1129-1175`):

- clones the monster's attack spell vector;
- allocates a lowercase monster name;
- looks the type up in the monster database;
- converts every defense spell node into a new `MonsterSpell` vector;
- creates `StdRng::from_entropy()`.

The 772 reference reads the already-loaded `RaceData.Spell` list during `IdleStimulus` (`crnonpl.cc:2521-2667`). Rebuilding immutable content is not required for parity. Spell-capable monsters repeat this work on every active idle cycle, and synchronized wakeups multiply it.

`monster_idle_try_talk` similarly clones the complete `Vec<String>` before a 1-in-50 gate (`idle_stimulus.rs:1974-1993`).

**Required change**

- Build one merged attack/defense `Vec<MonsterSpell>` in `MonsterAiConfig::from_monster_type` and store it on `Monster`.
- Store normalized type identity at spawn; do not lowercase names in the hot path.
- Store talk text in shared immutable data (`Arc<[String]>` or a monster-type key) or select the index before taking a short immutable borrow; do not clone every sentence for every gate.
- Replace per-pass entropy initialization with a persistent game-thread RNG dedicated to the applicable non-parity spell formulas. Verify RNG outcome requirements before changing the source/stream.

### IDLE-2 — Each terrain path search allocates and initializes fixed scratch

The 772 chase path allocates a boxed 529-cell array for every call and initializes it before scanning the viewport (`pathfinding.rs:644-744`). The linked-list open set also searches linearly on insertion/removal (`pathfinding.rs:525-640`), so one search has bounded but nontrivial `O(V²)` behavior with `V <= 441` active cells.

The bounded viewport and exact linked-list tie behavior are reference outcomes. Replacing it with a generic binary heap could change path choices. Allocation is not an outcome.

**Required change**

- Add reusable `TShortwayScratch` owned by the single game thread.
- Reinitialize only the active cells or use generation counters for visited fields.
- Keep the current linked-list ordering until differential path tests prove an alternative returns exactly the same paths and RNG consumption.
- Record searches, expanded cells, failed searches, and wall time per beat.

### IDLE-3 — Spatial searches allocate, sort, and deduplicate repeatedly

Target acquisition gathers candidates across several Z levels into a new vector, sorts by SlotMap key, and deduplicates (`idle_stimulus.rs:929-940`). Movement stimulus builds old/new spectator vectors, chains them into a third vector, sorts, and deduplicates (`monster_events.rs:62-132`).

These paths are bounded by the spatial index, but dense fights produce a move event for every step and can wake/repath many monsters. Sorting by creation key is already documented as a fallback that differs from the reference's 16×16 sector/chain order (`monster_events.rs:120-129`).

**Required change**

- Reuse game-thread scratch vectors.
- Add a generation-marked dedup set keyed by `CreatureId` to avoid sort/dedup where order is not observable.
- Where order is observable for 772, iterate/rebucket in the reference 16×16 sector order rather than sorting by ID.
- Record candidate count and recipient count; optimize only the high-cardinality call sites.

### Idle stimulus behavior that should not be “optimized” away

- Sleeping wild monsters intentionally have no recurring wakeup when no relevant creature keeps them awake (`idle_stimulus.rs:1024-1039`; `crnonpl.cc:2546-2556`).
- The 1000 ms fallback wait intentionally rechecks active monsters (`idle_stimulus.rs:1885-1898`; `crnonpl.cc:2925-2939`).
- Do not globally stagger these waits: equal-time ordering and decision timing are observable.

---

## ToDo scheduler

### What is already good

- The 1-indexed heap deliberately preserves the reference's structural equal-key order (`todo_queue.rs:1-121`). Keep it.
- `ToDoStart` clamps to at least `server_ms + 1`, preventing same-beat heap reinsertion (`creature_todo.rs:580-596`; `cract.cc:1010-1023`).
- The drain rechecks the creature's current `next_wakeup`, matching `Execute` semantics rather than requiring an exact stale-key match (`walk/mod.rs:411-428`; `cract.cc:783-786`).
- Per-creature `VecDeque` front/back operations are appropriate. The `has_*` scans are over normally tiny queues and are not currently a priority (`creature_todo.rs:162-205`).
- Do **not** call `shrink_to_fit` on every clear; that would turn retained capacity into allocator churn.

### TODO-1 — Recursive execution should be the reference-shaped loop

`run_monster_todo_execute` recursively calls itself for zero-delay chains, and `finish_creature_todo_execute` can recurse back into it (`idle_stimulus.rs:3521-3696`). The comments call this tail recursion, but Rust does not guarantee tail-call optimization. Player auto-walk length is protocol-bounded, yet Lua/content or future action producers can deepen the queue.

**Required change**

Rewrite the execution driver as an explicit loop with an enum describing:

- continue immediately;
- delayed/rearmed;
- action completed and idle requested;
- stop/error/dead.

This is both more idiomatic Rust and closer to `TCreature::Execute`'s `while (true)` (`cract.cc:783-898`). Add a diagnostic iteration guard that reports the creature and queue state, but do not silently defer valid zero-delay actions.

### TODO-2 — All-due draining creates a thundering herd, but is exact behavior

`drain_todo_queue` processes every due entry (`walk/mod.rs:392-429`). Many monsters naturally share a 1000 ms idle deadline; movement and damage stimuli can synchronize more. One due entry may then perform target search, spell evaluation, pathfinding, combat, map mutation, and packet fan-out.

This is the dominant scheduler risk, but adding a per-beat cap changes observable order and timing relative to `MoveCreatures` (`crmain.cc:1142-1157`). The correct first response is:

1. remove IDLE-1/IDLE-2 allocation and DEC/packet amplification;
2. bound ingress so beats run;
3. instrument due count and oldest lateness;
4. load-test synchronized populations.

Only introduce a scheduler budget as an explicit overload-mode deviation, with a documented fairness and lateness policy.

### Stale ToDo entries

A creature can have obsolete heap entries because clearing or rescheduling updates `next_wakeup` without removing old heap nodes. The drain skips entries whose current wakeup is absent/not due (`walk/mod.rs:411-428`). This behavior is tied to the reference's insert-and-recheck model. Do not convert it to an indexed single-entry scheduler without differential tests: an old popped entry may legitimately execute a newer wakeup that is already due.

Add metrics first:

- heap entries;
- creatures with a current wakeup;
- popped entries;
- popped entries that execute;
- skipped stale entries;
- maximum and percentile lateness.

If the stale ratio is high in production, compact only at a safe beat boundary and prove drain order is unchanged for all live entries.

---

## Observability required before optimization claims

Current `advance_beat` timing warns only when the call reaches 100 ms or lag is already severe (`game_world_tick.rs:93-113`). Keep it, but add low-overhead aggregated counters/histograms.

### Per loop turn

- command queue depth and oldest command age;
- commands processed before yielding to a beat;
- beat scheduled time, start lateness, and wall duration;
- missed/coalesced beat count;
- output queued bytes per connection and maximum writer age;
- login load latency and concurrent login loads.

### Per beat subsystem

- `creatures_us`, `cron_us`, `skills_us`, `other_us`, `todo_us` histograms;
- count processed by each subsystem;
- ToDo heap/current-wakeup/stale-pop/due/lateness values;
- idle passes, target candidates, move-stimulus recipients;
- path searches, expanded cells, failures, and pathfinding wall time;
- decay live entries, heap entries, stale pops, due count, transformed/removed descendants;
- packets and bytes generated by one decay/idle action.

Use periodic aggregated `tracing` events first; avoid per-action INFO logs in hot paths.

## Remediation order

### Phase 0 — Baseline and regression harness — **Code done; live capture open**

1. ~~Expand OBS-1 beyond thin warn-path counters~~ — `obs.rs` + instrumentation.
2. Capture 60-second baselines for idle world, dense spawn, active chase, spell-heavy fight, corpse wave, and packet flood — template [`GAME_LOOP_OBS_BASELINES.md`](GAME_LOOP_OBS_BASELINES.md).
3. Record p50/p95/p99 beat lateness and subsystem wall time, not only average throughput.

### Phase 1 — Protect the game thread — **Done**

1. ~~Split async login load from game-thread insertion (GL-1).~~
2. ~~Bound command ingress and enforce a command slice before servicing beats (GL-2).~~
3. ~~Bound output queues and remove the blocking registry mutex from beat flush (GL-3).~~

### Phase 2 — Make decay cost proportional to actual changed items — **Done**

1. ~~Use item parent for equipment expiry.~~
2. ~~Batch container empty bookkeeping and use incremental aggregate deltas.~~
3. ~~Replace recursive item-tree destruction.~~
4. ~~Replace lazy cancellation with an indexed heap.~~
5. ~~Separate 772 cron time from the lag-skipped movement clock.~~

### Phase 3 — Remove idle/path allocation churn — **Mostly done**

1. ~~Prebuild merged monster spells and normalized type identity.~~
2. ~~Stop cloning talk text.~~
3. ~~Reuse `TShortway` and spectator scratch.~~
4. Preserve reference ordering with differential tests — **partial** (path scratch reuse parity test exists; 16×16 spectator sector order still open under IDLE-3).

### Phase 4 — Harden ToDo execution — **Code done; overload budget open**

1. ~~Replace recursive execution with an explicit loop.~~
2. ~~Add stale/due/lateness metrics~~ — OBS-1 (`todo_popped` / `todo_executed` / `todo_stale` / lateness histograms).
3. Decide whether an explicit overload budget is necessary only after profiling the optimized path — **TODO-2 deferred**.

---

## Next steps

Ordered for the next pass. **Do not start TODO-2** until steps 2–4 produce load data that still shows synchronized all-due drain as the bottleneck.

### 0. Manual smoke (before more code)

Confirm the post-landing session fixes on a live client after `./scripts/run_server.sh` restart:

| Check | Expected |
|---|---|
| Floor change onto a dense monster floor | No OTClient desync; monsters can acquire/chase without wedging the game thread |
| Logout (`0x14`) | Immediate return to character list (TCP closes); no ~10s hang |
| Ctrl+C | Graceful flush or force-exit within 10s / second Ctrl+C; no `block_in_place` panic |
| Failed login / logout mid-async-load | Game TCP closes; no half-open session |

Checklist mirrors `tasks/todo.md` live retest items.

### 1. Phase 0 + full OBS-1 (highest engineering priority) — **code done**

~~Finish observability as aggregated counters/histograms~~ — landed. Operator capture:

| Area | Status |
|---|---|
| Loop turn / output / subsystems / ToDo / idle-path / decay | **Instrumented** → `RUST_LOG=tfs_obs=info` |
| 60s baselines | Template: [`GAME_LOOP_OBS_BASELINES.md`](GAME_LOOP_OBS_BASELINES.md) — fill from live runs |

Then capture **60-second baselines** (idle, dense spawn, chase, spell fight, corpse wave, packet flood) and record p50/p95/p99 beat lateness + subsystem wall time.

### 2. Close partial findings

| ID | Next work |
|---|---|
| IDLE-3 | Where 772 order is observable, iterate/rebucket in 16×16 sector order instead of SlotMap-key sort; generation-marked dedup where order is not observable |
| GL-4 | Optional active indexes (players / conditioned / dead-pending) if scratch Vecs are insufficient under load — do **not** add an extra full scan just to size vectors |
| Beat startup | Parity test for immediate `interval` first tick vs reference waiting for alarm; use `interval_at(now + beat, beat)` if the immediate advance is unintended |
| Failed-login UX (low) | Optional gameworld `disconnectClient` error string before TCP close (client currently just drops) |

### 3. Strengthen required tests (full scale)

Thin coverage exists for several items; bring the suite up to the audit bar:

| # | Test | Current | Next |
|---|---|---|---|
| 1 | Command starvation | Budget-yield unit test | Multi-second flood; assert max beat lateness bound |
| 2 | Backpressure | Game-lane full + outbound unit tests | Deterministic disconnect/shed under filled command + output queues |
| 3 | Slow login DB | Beats continue while load pending | Also assert decay + ToDo continue under delayed `load_player_full` |
| 4 | Decay churn | Heap ∝ live entries | Keep; extend duration/churn volume if needed |
| 5 | Decay burst | Small burst / order tests | Thousands of tile/inv/container/nested corpse expiries; packets + wall-time bound |
| 6 | Lag/decay parity | Present (`lag_guard` + decay clock + `item_decay_remaining_ms`) | Keep as regression |
| 7 | Spell idle burst | Spawn merge unit test | Many spell-capable monsters; assert no per-pass rebuild + measure alloc/time |
| 8 | Path burst | Scratch reuse + storm-reuse + expand_next gen fix | Many concurrent repaths + existing 772 oracle fixtures |
| 9 | Deep zero-delay ToDo | Present + guard re-arm test | Keep as regression |
| 10 | Stale ToDo entries | Metrics still thin | Randomized schedule/clear/reschedule differential vs reference heap / `NextWakeup` |

### 4. Gate: load-test, then maybe TODO-2

1. Run production-shaped load with Phases 1–4 + OBS-1 baselines.
2. If synchronized all-due ToDo/cron still dominates **after** amplification is gone, design an **explicit** overload budget (fairness + lateness policy documented) — that is **TODO-2**.
3. Until then: preserve all-due drain semantics; do not silently cap.

### 5. Verification gate (still open)

```bash
rtk cargo test -p tfs-rust-core --lib
rtk cargo clippy -p tfs-rust-core --all-targets -- -D warnings
rtk cargo test --workspace
```

---

## Required tests and benchmarks

Status relative to the original list:

1. **Command starvation:** partial — budget unit test; needs multi-second lateness bound.
2. **Backpressure:** partial — lane-full shed / outbound re-queue unit tests; needs shed determinism under dual fill.
3. **Slow login DB:** partial — beats continue; extend to decay/ToDo continuity.
4. **Decay churn:** **done** (heap ∝ live).
5. **Decay burst:** partial — needs thousands-scale + packet/time bounds.
6. **Lag/decay parity:** **done** (includes round-clock look/save remaining).
7. **Spell idle burst:** partial — needs multi-monster alloc/time assertion.
8. **Path burst:** partial — scratch reuse + storm-reuse; needs multi-repath burst.
9. **Deep zero-delay ToDo:** **done** (includes iteration-guard re-arm).
10. **Stale ToDo entries:** **open**.

Suggested verification commands:

```bash
rtk cargo test -p tfs-rust-core --lib
rtk cargo clippy -p tfs-rust-core --all-targets -- -D warnings
rtk cargo test --workspace
```

## Bottom line

The original diagnosis stands: lag came from **admission and amplification** on the game thread, not from `O(log n)` heap ops. **Protect-the-loop and decay/idle/ToDo code fixes are in**, and the first wave of **session/decay/path regressions from that landing is fixed** (`68b7c93`).

**Immediate next work:** (0) manual smoke retests → (1) **fill live OBS baselines** → (2) close **IDLE-3 / GL-4 / full-scale tests** → (3) load-test → only then consider **TODO-2**.