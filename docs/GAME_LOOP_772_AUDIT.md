# 772 Game Loop & Scheduler — 1098-Bleed Parity Audit

Scope: a *structural* pass over the Rust game loop and creature/action scheduling against the
CipSoft 772 beat-driven model (`reference/cipsoft-772/tibia-game-master/src/`). The question this
audit answers: **is TFS 1098 reactive-scheduler logic bleeding into the 772 beat-driven loop?**
This is **not** a line-by-line trace — it flags places where the Rust shape uses a 1098 mechanism
(wall-clock timers, per-step `nextAction` lockout, 50 ms tick) where the 772 authority drives
everything off one logical clock (`ServerMilliseconds`) and the `ToDoQueue`, drained once per beat.

Reference functions (772 loop authority = **mechanics** tree, `tibia-game-master/src/`):
- `LaunchGame` / `AdvanceGame` — `main.cc:318–461`, `462–500`
- `MoveCreatures` — `crmain.cc:1142–1158`
- `Beat` constant + beat timer — `config.cc:102`, `main.cc:147–171`
- Subsystem cadence (`CreatureTimeCounter` 1750, `Cron` 1500, `Skill` 1250, `Other` 1000) — `main.cc:319–440`
- Per-action timing — `TCreature::Earliest*Time` (`EarliestWalkTime`, `EarliestSpellTime`,
  `EarliestMultiuseTime`) — `cr.hh` / `crmain.cc:924`

Wire keepalive is **not** loop authority — `0x1D`/`0x1E` ping genuinely exists in the 772 wire
(TVP `gameserver/src/protocolgame.cpp:466–468`, `sendPing`/`sendPingBack` `:1516–1534`), so the
periodic ping is **not** a 1098 bleed (see "Ruled out" below).

Rust under audit: `game_loop.rs`, `game_world_tick.rs`, `subsystem_counters_772.rs`,
`walk/mod.rs` (scheduler primitives), `walk_action.rs`, `creature_think.rs`, `player_ping.rs`,
`run_server.rs`.

---

## Verdict

The macro loop architecture is faithful and cleanly era-split. The 1098 reactive scheduler
(Dispatcher + per-creature `tokio::time::sleep_until` walk wakes + 50 ms `on_tick`) and the 772
beat loop (`AdvanceGame` + `ToDoQueue` on `ServerMilliseconds`) are selected at startup and do not
share a code path at the loop level:

- **Loop selection** (`run_server.rs`): `beat_driven ⇒ run_game_loop_772`, else
  `run_game_loop_1098`. `walk_wake_tx` is `None` under 772, so the 1098 walk-wake channel is never
  armed and `run_game_loop_772` never selects on it. Correct.
- **Flush policy**: 772 uses `FlushPolicy::BeatEndOnly` (buffer → beat-end `SendAll`); 1098 uses
  `ImmediateOnMovement`. `needs_immediate_flush` short-circuits to `false` under `BeatEndOnly`.
  Matches `AdvanceGame` ending in one `SendAll()`. Correct.
- **`advance_beat_772`** mirrors `AdvanceGame`: staggered subsystem counters (1750/1500/1250/1000),
  then `server_ms += delay`, then `drain_todo_queue()` ≡ `MoveCreatures`
  (`while ToDoQueue.top.Key <= ServerMilliseconds { Execute() }`). Correct in shape.
- **`on_tick` / `check_creatures`** (the 1098 100 ms bucketed `Game::checkCreatures` sweep) are
  reachable **only** from `run_game_loop_1098`. The 772 path uses `process_creatures_772` on the
  `fired.creatures` 1 Hz subsystem. No bleed.
- **Walk scheduler** is era-split at the leaf: `add_event_walk` / `schedule_walk_followup_deadline`
  route 772 through `schedule_creature_wakeup(server_ms, …)` → `ToDoQueue`, and
  `commit_next_walk_deadline` (the wall-clock 1098 path) **early-returns when `beat_driven_loop`**
  (`walk/mod.rs:627`). So the per-creature wall-clock walk timer cannot fire under 772. No bleed.

The bleeds that remain are not in the loop skeleton — they are in the **player action/scheduler-task
layer**, which is still 100% TFS 1098 and runs unconditionally inside the 772 beat. Two are
behavioral; the rest are fidelity/cleanliness.

---

## Findings (ranked by likely impact)

### 1. `walk_action` (walk-then-act) is a TFS 1098 wall-clock scheduler task running inside the 772 beat — HIGH

`walk_action.rs` is the TFS `Player::setNextWalkActionTask` / `createSchedulerTask(400, …)`
mechanism: when a player uses/moves an item that needs an approach walk, the action is stored and
fired `WALK_ACTION_DELAY = 400 ms` after `onWalkComplete`, scheduled on the **wall clock**:

```rust
// walk_action.rs
pub const WALK_ACTION_DELAY: Duration = Duration::from_millis(400);
p.walk_action_due = Some(now + WALK_ACTION_DELAY);          // wall-clock Instant
```

It is drained from the 772 beat with a **wall-clock** `Instant::now()`:

```rust
// game_world_tick.rs::advance_beat_772
self.process_walk_action_tasks(Instant::now());            // not server_ms
```

```rust
// walk_action.rs::process_walk_action_tasks
let due_at = p.walk_action_due?;
(now >= due_at).then_some((cid, action))                   // compares wall clock
```

CipSoft 772 has **no** separate wall-clock scheduler task for this. A player's "go to X then act"
is a sequence of `ToDo` entries (`ToDoGo` waypoints then the action `ToDo`) on the **`ToDoQueue`**,
keyed on `ServerMilliseconds` and drained by `MoveCreatures` like everything else
(`crmain.cc:1142`). There is one clock (`ServerMilliseconds`) and one scheduler (`ToDoQueue`).

**Why it's a real bleed:** the 772 loop now has *two* clocks driving creature behavior — the logical
`server_ms`/`ToDoQueue` and a parallel wall-clock `walk_action_due` evaluated every beat. The 400 ms
delay and `nextAction` reschedule (`defer_player_walk_action`) are pure 1098. On the beat grid the
fire time also quantizes oddly (a 400 ms wall delay observed only at 200 ms beat boundaries), and it
can fire on a different beat than the equivalent `ToDo` entry would, desyncing player action timing
from the decompile.

**Recommendation:** under `beat_driven_loop`, model the deferred action as a `ToDo`/wakeup on
`server_ms` (enqueue via the same `schedule_creature_wakeup` / `ToDoQueue` path the walk uses), not a
wall-clock `walk_action_due`. Keep the 1098 `walk_action_due` path for the 1098 loop only.

### 2. `nextAction` per-step action lockout is 1098; 772 uses per-action `Earliest*Time` — MEDIUM/HIGH

Every player step sets the TFS 1098 `nextAction` instant in the shared move path:

```rust
// walk/mod.rs  (runs for both eras)
let dur_ms = get_step_duration_ms_with_direction(k, k.base(), direction, gs_next_action, &self.mechanics);
if let CreatureKind::Player(p) = k {
    p.next_action_until = Some(now + Duration::from_millis(dur_ms.max(1) as u64));   // wall clock
}
```

and the game-loop packet gate enforces it in **both** loops:

```rust
// game_loop.rs::handle_game_packet
if game_packet_requires_timed_action(&packet) && !world.player_timed_action_ready(cid, now) {
    return;   // TFS Player::canDoAction / nextAction
}
// player.rs
pub fn timed_action_ready(&self, now: Instant) -> bool { self.next_action_until.is_none_or(|t| now >= t) }
```

This is TFS 1098 `Player::canDoAction` / `nextAction` (`player.cpp`): after each walk step, *all*
gated actions (use/attack/trade/…) are blocked for one step-duration, on the wall clock.

CipSoft 772 does **not** gate actions behind a unified post-walk lockout. It uses **independent
per-action** timestamps on `ServerMilliseconds` — `EarliestWalkTime`, `EarliestSpellTime`,
`EarliestMultiuseTime` (e.g. `crmain.cc:924` checks `EarliestAttackTime`). Walking advances
`EarliestWalkTime`, not a global action lock; using an object advances `EarliestMultiuseTime`. So a
772 player who walks and immediately uses an object is **not** blocked the way the 1098 `nextAction`
gate blocks them here.

**Effect:** in the 772 loop, action availability after walking follows the 1098 rule, not the
CipSoft per-timer rule — wrong cadence for the walk→use / walk→attack interaction, and it's measured
on the wall clock rather than `server_ms`.

**Recommendation:** gate 772 actions on the relevant `Earliest*Time` (logical `server_ms`), not the
1098 `next_action_until`. At minimum, scope `next_action_until` setting + the `handle_game_packet`
`timed_action_ready` gate to `!beat_driven_loop`.

### 3. Beat cadence: per-beat `Burst` replay + always-drain vs CipSoft coalesced `AdvanceGame(N·Beat)` + lag-guard — MEDIUM (772-internal, not 1098)

The Rust loop uses tokio `interval` with `MissedTickBehavior::Burst` and advances **one beat per
fire**:

```rust
// game_loop.rs::run_game_loop_772
let mut beat_timer = interval(Duration::from_millis(beat_ms));
beat_timer.set_missed_tick_behavior(MissedTickBehavior::Burst);
// ...
_ = beat_timer.tick() => { world.advance_beat_772(beat_ms); flush_pending_outgoing(...); }
```

CipSoft coalesces all missed beats into a single call and **skips creature movement under lag**:

```cpp
// main.cc:493 LaunchGame
int NumBeats = SigAlarmCounter;
if(NumBeats > 0){ SigAlarmCounter = 0; AdvanceGame(NumBeats * Beat); }   // one call, Delay = N·Beat
// main.cc:446 AdvanceGame
if(Delay < 1000){ MoveCreatures(Delay); Lag = false; }                   // lag-guard: skip if behind
else { Lag = true; /* no MoveCreatures this round */ }
```

Two divergences:
- **No lag-guard.** `advance_beat_772` always calls `drain_todo_queue()`. CipSoft skips
  `MoveCreatures` when `Delay >= 1000 ms`. The code comment "C++ `MoveCreatures` always drains — no
  lag-catchup skip (`crmain.cc:1106`)" conflates two layers: `MoveCreatures` *itself* always drains,
  but `AdvanceGame` *only calls it* when `Delay < 1000`. The guard is missing.
- **Per-beat vs coalesced.** With `Burst`, N missed beats become N× `advance_beat_772(200)`. CipSoft
  does 1× `AdvanceGame(N·200)`. For the subsystem counters this differs: CipSoft fires each
  subsystem **at most once** per `AdvanceGame` (single `-= 1000`), so a coalesced 2000 ms catch-up
  runs each subsystem once and `server_ms` jumps 2000 in one `MoveCreatures`; the Rust version steps
  the counters and the `ToDoQueue` in 200 ms increments. Observable creature scheduling can differ
  during catch-up.

This is a 772-vs-772 fidelity gap (not 1098 bleed), but it sits in the scheduler and changes
behavior under lag. **Recommendation:** coalesce pending beats into one `advance_beat_772(n·beat)`
and add the `Delay < 1000` `MoveCreatures` skip.

### 4. `tick_counter` is a 1098 50 ms-tick artifact retrofitted onto the beat — LOW

`advance_beat_772` keeps a 50 ms tick counter alive purely to feed two 1098-shaped consumers:

```rust
self.tick_counter = self.tick_counter.saturating_add(delay_ms / 50);   // 50 ms cadence
if fired.skills { let _ = self.decay.tick(self.tick_counter); }        // decay keyed on tick_counter
// run_other_subsystems(.., false) → events.lua_gc_step() every call (ok)
```

`tick_counter` is the 1098 `on_tick` (~50 ms) counter (`game_world_tick.rs:19`). Decay is driven by
it in both eras, so item decay cadence is expressed in 50 ms ticks rather than the 772 logical
subsystem clock. CipSoft drives object/decay work from its cron/skill subsystems on
`ServerMilliseconds`, not a 50 ms tick. Functionally close (decay is coarse), but it's a 1098 unit
leaking into the 772 advance. **Recommendation:** express decay in logical ms / the `fired.skills`
subsystem directly; drop the `delay_ms / 50` proxy from the 772 path.

### 5. Turn look-ahead coalescing in `handle_game_packet` — INFO (benign under 772)

The `Turn` handler peeks the next command (`cmd_rx.try_recv()`) to coalesce turn+move and flush a
deferred `0x6B` (`flush_deferred_turn_broadcast`). This is a 1098 walk-smoothness construct
(`tasks/walk-smoothness-audit.md` Bug 7) and runs in both loops. Under `BeatEndOnly` it does not
force an early wire flush, so it's benign for 772 framing — but the deferred-turn broadcast and the
one-command peek are 1098-era smoothing that 772 (no client-side prediction) doesn't need. Confirm
it doesn't reorder a `ToDo`-relevant packet across a beat boundary; otherwise leave as shared.

---

## Ruled out (checked, not a bleed)

- **Periodic ping (`tick_player_pings`, `0x1D`/`0x1E`).** Ping is part of the **772 wire** — TVP
  `gameserver/src/protocolgame.cpp:466–468` parses `0x1D`/`0x1E`, and `sendPing` (`:1516`) emits
  `0x1D` for OTClient / `0x1E` otherwise. Running it in the 772 `Other` subsystem is correct, not a
  1098 leak. (Separate, already-tracked issue: `send_ping()` is hardcoded `0x1E` and not
  OTClient-aware — see `docs/772_OTCLIENT_PARITY.md` "Server ping `0x1D`" GAP. That's a wire-codec
  bug, not a loop bleed.)
- **`on_tick` / `check_creatures` (1098 bucketed think).** Reachable only from
  `run_game_loop_1098`; the 772 sweep is `process_creatures_772` on the 1 Hz `fired.creatures`
  subsystem.
- **Per-creature wall-clock walk wake (`walk_wake_rx`, `commit_next_walk_deadline`,
  `process_walk_deadlines`).** `walk_wake_tx` is `None` under 772; `commit_next_walk_deadline`
  early-returns when `beat_driven_loop`; `handle_game_packet` gates `process_walk_deadlines` behind
  `!beat_driven_loop`. The 772 walk path uses `schedule_creature_wakeup` → `ToDoQueue` exclusively.
- **Subsystem cadence.** `subsystem_counters_772` thresholds (1750/1500/1250/1000, `-= 1000` on
  fire) match `AdvanceGame` exactly.
- **Monster push / think / look era-gating.** Already branch on `beat_driven_loop`
  (`monster_push.rs`, `monster_targets.rs`, `creature_think.rs`) — out of scope here; covered by
  `MONSTER_AI_772_AUDIT.md`.

---

## Recommended fix order

1. **Route the deferred walk-action through the 772 `ToDoQueue`** (Finding 1) — biggest behavioral
   bleed; removes the parallel wall-clock task from the beat loop.
2. **Scope `next_action_until` + the `timed_action_ready` packet gate to 1098** and gate 772 actions
   on `Earliest*Time` / `server_ms` (Finding 2).
3. **Coalesce missed beats + add the `Delay < 1000` lag-guard**, and correct the misleading
   "always drains" comment (Finding 3).
4. **Drop the `delay_ms / 50` `tick_counter` proxy** from the 772 advance; drive decay off logical
   ms / `fired.skills` (Finding 4).
5. Confirm the Turn look-ahead never reorders a `ToDo`-relevant packet across a beat (Finding 5).

## Repro suggestions

- **Finding 1/2:** player walks one tile then immediately uses an item (e.g. a rune/ladder) — diff
  the action fire time and availability vs the C++ oracle on the 200 ms grid; the 400 ms wall task
  and `nextAction` lockout should both show up.
- **Finding 3:** stall the game thread > 1 s (debugger / heavy load) and compare creature movement:
  CipSoft drops a round (`Lag`) and coalesces; the Rust loop replays each beat and always drains.

## Reference index

| Behavior | C++ 772 ref | Rust | Status |
|---|---|---|---|
| Beat constant / timer | `config.cc:102`, `main.cc:147` | `formulas` `beat_ms=200`, `run_game_loop_772` interval | faithful |
| Main loop / coalesce / lag-guard | `main.cc:493`, `446` `AdvanceGame` | `run_game_loop_772` (`Burst`, per-beat, always-drain) | **Finding 3** |
| Subsystem cadence | `main.cc:319–440` | `subsystem_counters_772.rs` | faithful |
| `MoveCreatures` drain | `crmain.cc:1142` | `advance_beat_772` → `drain_todo_queue` | faithful |
| Logical clock | `ServerMilliseconds` | `server_ms` | faithful |
| Per-creature walk schedule | `cract.cc` `ToDoStart`/`NotifyGo` | `schedule_creature_wakeup` → `ToDoQueue` | faithful |
| Walk-then-act | `ToDo` chain on `ServerMilliseconds` | `walk_action.rs` wall-clock `walk_action_due` (1098) | **Finding 1** |
| Action timing gate | `Earliest*Time` (`crmain.cc:924`) | `next_action_until` + `timed_action_ready` (1098) | **Finding 2** |
| Decay cadence | cron/skill subsystem (logical) | `decay.tick(tick_counter)` (50 ms proxy) | **Finding 4** |
| Ping keepalive | `protocolgame.cpp:466/1516` (772 wire) | `player_ping.rs` (`Other` subsystem) | ruled out (wire) |
| Bucketed think | — (1098 only) | `check_creatures` in `on_tick` (1098 path only) | ruled out |

*Audit only — no game-loop code modified. Line-level tracing of the `walk_action`/`Earliest*Time`
port is the follow-up.*

---

# Pass 2 — ToDoQueue + `Execute` internals (the foundation)

Pass 1 covered the loop skeleton and the action/scheduler-task layer. This pass goes one level
down — the `ToDoQueue` data structure and `TCreature::Execute` — because if the queue ordering and
the execute step aren't faithful, no amount of correct monster-AI decision logic on top will match
the oracle: **the AI draws RNG in drain order, and drains in heap order.** Findings here are more
fundamental than anything in Pass 1.

Reference: `containers.hh:144–227` (`priority_queue`), `cract.cc:783–905` (`Execute`),
`cract.cc:906–951` (`CalculateDelay`), `cract.cc:1010–1031` (`ToDoStart`/`ToDoYield`),
`crmain.cc:1142–1158` (`MoveCreatures`).
Rust: `todo_queue.rs`, `walk/mod.rs::drain_todo_queue` / `process_creature_todo`,
`idle_stimulus.rs::run_monster_todo_execute`, `monster_ai.rs::rescue_stalled_chase_monsters_772`.

## Finding 6. ToDoQueue equal-key tie order does not match CipSoft's structural heap — CRITICAL

CipSoft's `ToDoQueue` is a hand-rolled binary min-heap with **no secondary key**
(`containers.hh:150`). Insert sifts up with `if(Parent->Key <= Current->Key) break;`
(`:171`) and `deleteMin` sifts down picking the left child on ties
(`if(Other->Key < Smallest->Key)` — strict, `:204`). So when several creatures share the same
`ExecutionTime` (the common case — everything is quantized to the 200 ms beat grid), the pop order
is the **implicit array layout** produced by those sift operations. It is fully deterministic but
it is **neither FIFO nor LIFO** — it depends on the heap's structural shape at that moment.

Rust uses `BinaryHeap<Reverse<ToDoEntry>>` with an explicit `sequence` field as a secondary key:

```rust
// todo_queue.rs
fn cmp(&self, other) -> Ordering {
    self.execution_time.cmp(&other.execution_time)
        .then_with(|| self.sequence.cmp(&other.sequence))   // FIFO tie — NOT in C++
}
```

Two problems compound:
1. **`std::BinaryHeap` is a different heap** (different sift internals than CipSoft's array heap),
   so even *without* a secondary key its equal-key order would not match CipSoft.
2. **The `sequence` tie forces FIFO-on-ties**, which is a *specific wrong answer* — CipSoft's order
   is structural, not insertion-order.

**The smoking gun is in the file itself:** `harness_go_step_tie`, `harness_appear_idle_tie`, and
`harness_go_step_tie_realmap_bowl` hand-map `spawn_order → magic tie index` to reproduce the C++
drain order for *specific* fixtures (cyclops quad, real-map bowl dual, appear-LIFO). These exist
**because** FIFO `sequence` does not match the oracle, so each scenario was reverse-engineered and
pinned:

```rust
pub fn harness_go_step_tie(spawn_order: u16) -> u64 {
    match spawn_order { 4 => 0, 3 => 1, 1 => 2, 2 => 3, n => u64::from(n) } // NW, S, far-N, E
}
```

These maps cover only the hardcoded fixtures. **Any equal-`server_ms` multi-creature situation that
isn't one of those fixtures drains in FIFO order, which is the wrong order**, and since idle/AI RNG
is drawn at drain time, the glibc stream desyncs from the oracle for the rest of those creatures'
lives. This is the same class of defect as the monster-AI RNG findings, but one layer deeper — it
poisons *every* multi-creature beat, not just pushes.

**Recommendation (foundational):** port CipSoft's `priority_queue` **verbatim** — the same
array-backed binary heap, the same `insert` sift-up (`Parent->Key <= Current->Key` break) and
`deleteMin` sift-down (left-child bias on ties), keyed on `ExecutionTime` only, `Data = CreatureId`.
Then **delete** the `sequence` field and all three `harness_*_tie` maps: the correct equal-key order
emerges structurally and matches the oracle for *all* scenarios, not just the pinned ones. This is
~40 lines and removes a whole category of parity hacks.

## Finding 7. `Execute` runs one ToDo action per heap pop; CipSoft drains all zero-delay entries atomically — HIGH

CipSoft `TCreature::Execute` (`cract.cc:783`) is a `while(true)` loop:

```cpp
while(true){
    if(!LockToDo || IsDead || NextWakeup > ServerMilliseconds) break;
    if(NrToDo <= ActToDo){ ToDoClear(); IdleStimulus(); break; }   // queue drained → idle
    uint32 Delay = CalculateDelay();                                // Earliest*Time vs ServerMs
    if(Delay > 0){ NextWakeup = ServerMs + Delay; ToDoQueue.insert(NextWakeup, ID); break; }
    TD = ToDoList[ActToDo++]; /* execute Go/Rotate/Move/Use/Attack/Talk */  // Delay==0 → run NOW
}                                                                   // loop: next entry same beat
```

So a creature's **consecutive ready (`Delay==0`) entries run back-to-back in a single `Execute`
call**, atomically with respect to other creatures. `CalculateDelay` (`cract.cc:906`) returns 0
whenever the relevant `Earliest*Time` is already ≤ `ServerMilliseconds` (e.g. a `Rotate` then a
ready `Use`, or `TDAttack` when `EarliestAttackTime` has passed). Only a future `Earliest*Time` /
`Wait` (`Delay>0`) re-inserts and ends the call.

Rust `run_monster_todo_execute` is documented "**Run one queued action**" — it executes a single
action then `finish_creature_todo_execute` (re-arm) or idle. Multi-entry zero-delay chains are
therefore spread across multiple heap pops (each re-inserting at `server_ms`), and those pops
**interleave with other creatures' same-timestamp entries**. Concretely, if creature A has
`[Rotate(0), Use(0)]` and B has `[Go(0)]` all at time T, CipSoft runs `A.Rotate, A.Use` (atomic)
then `B.Go`; Rust can run `A.Rotate, B.Go, A.Use`. Different execution order **and** different RNG
draw order.

**Recommendation:** model `Execute` as the synchronous loop over the per-creature ToDo list within
one drain visit — process entries while `CalculateDelay()==0`, break + re-insert on the first
`Delay>0`, `IdleStimulus` on empty. One heap visit = one full `Execute`, matching the oracle's
atomicity.

## Finding 8. `rescue_stalled_chase_monsters_772` is a non-CipSoft band-aid for a scheduling gap — HIGH

`drain_todo_queue` ends every beat with:

```rust
self.rescue_stalled_chase_monsters_772();   // "Per-beat safety net — rescue chase monsters
                                            //  stranded without a heap wakeup."
```

which scans **all** non-idle, non-fleeing monsters and reschedules any that are
`monster_chase_stalled_without_wakeup` / `monster_combat_scheduler_needs_refresh`. CipSoft has no
such pass: `Execute`/`ToDoStart` guarantee a creature always either re-inserts into the queue or
goes to `IdleStimulus` (which itself queues and `ToDoStart`s). A creature **cannot** end up
"stranded without a heap wakeup" in the oracle.

The existence of this rescue means a Rust code path lets a monster finish a beat with
`next_wakeup == None` and an empty/blocked ToDo queue — most likely the `front_is_go` synchronous-go
suppression branch in `process_creature_todo` (which `return`s after `cleanup()` without always
arming a wakeup), or a `take()` of `next_wakeup` without a matching re-insert. The rescue masks that
bug but at an arbitrary reschedule point and arbitrary order (full `creatures.iter()` scan), so it
**introduces** its own timing/order divergence and is O(monsters) every beat at 2000+ players.

**Recommendation:** treat the rescue as a failing assertion, not a feature. Find the arming gap
(once Finding 7's `Execute` loop is faithful and Finding 6's heap is exact, the most likely culprits
collapse), guarantee every `process_creature_todo` exit either re-inserts or idles, then **delete**
`rescue_stalled_chase_monsters_772`. A correct scheduler never strands a creature.

## Finding 9. Stale-entry filter uses `next_wakeup == execution_time`; CipSoft uses `NextWakeup > ServerMilliseconds` — MEDIUM

Both engines keep stale entries in the heap (neither does decrease-key; `ToDoStart`/`Execute` always
`insert`). They differ in how a popped entry is validated:

```rust
// drain_todo_queue — process only on EXACT match
let still_valid = self.creatures.get(id).and_then(|k| k.base().next_wakeup) == Some(entry.execution_time);
if still_valid { self.process_creature_todo(id); }
```

```cpp
// Execute — process whenever due, regardless of exact match
if(NextWakeup > ServerMilliseconds) break;   // else fall through and run
```

CipSoft runs the creature if its current `NextWakeup <= ServerMilliseconds` even if the popped
entry's key isn't the creature's *latest* scheduled time (a creature can be rescheduled earlier and
still execute on an older due entry). The Rust `==` filter discards any popped entry whose key isn't
exactly the creature's current `next_wakeup`, and relies on **every** scheduler write keeping
`next_wakeup` perfectly in lockstep with the inserted key. Where they diverge (re-arm to a different
key in the same beat, or two inserts collapsing), Rust can drop an entry CipSoft would have run —
plausibly another contributor to the Finding 8 "stranded" cases.

**Recommendation:** switch the filter to the oracle's semantics — pop, then run iff
`next_wakeup <= server_ms` (and let the faithful `Execute` loop's own `NextWakeup > ServerMilliseconds`
check handle re-deferral), rather than exact-key matching.

## Finding 10. `MAX_DRAINS_PER_BEAT = 4096` cap has no oracle equivalent — MEDIUM

```rust
const MAX_DRAINS_PER_BEAT: usize = 4096;
while drained < MAX_DRAINS_PER_BEAT { ... }
```

`MoveCreatures` (`crmain.cc:1144`) drains unconditionally: `while(ToDoQueue.Entries > 0 && top.Key <= ServerMilliseconds)`.
With many same-beat wakeups (large populations, or a catch-up `server_ms` jump), the Rust cap stops
early and leaves due entries for the next beat — a 200 ms-plus deferral the oracle never does, and a
silent order perturbation. **Recommendation:** drain unconditionally to match; if a safety valve is
desired for runaway loops, make it a logged hard error, not a routine early-exit.

## Finding 11. Per-creature `cleanup()` inside the drain loop — LOW / verify

`process_creature_todo` calls `self.cleanup()` (TFS `Game::cleanup` — drains
`creatures_pending_release` / `items_pending_release`) after each creature's execute. CipSoft
`MoveCreatures` does not run a cleanup between Executes. In practice this looks benign — `cleanup`
only removes entities already marked for release, and a dead creature's later heap entry is dropped
by the validity filter / `IsDead` break — but it does remove entities mid-beat between other
creatures' Executes. **Recommendation:** confirm release ordering is observationally neutral, or
hoist `cleanup` to end-of-drain to match `MoveCreatures` granularity.

## Revised foundation picture

The Pass-1 bleeds (wall-clock `walk_action`, `nextAction` gate) sit *on top of* a queue whose
**equal-key ordering is itself wrong outside the pinned fixtures** (Finding 6) and whose **execute
step isn't atomic** (Finding 7). Those two are the load-bearing issues: they change cross-creature
execution and RNG-draw order on essentially every multi-creature beat, which is exactly why
"the monster AI passes the sim but isn't right" — the sim fixtures are the ones with hand-pinned
ties. The `rescue` pass (Finding 8) and the `==` filter (Finding 9) are symptoms/contributors of the
same scheduler-arming fragility.

## Recommended fix order (foundation first)

1. **Port CipSoft `priority_queue` verbatim** (Finding 6) — array heap, key-only, exact sift; delete
   `sequence` + all `harness_*_tie` maps. Re-run the harness: ties should now match *without* the
   hardcoded maps.
2. **Make `Execute` the synchronous zero-delay loop** (Finding 7) — one heap visit drains a
   creature's ready ToDo chain; break/re-insert on first `Delay>0`.
3. **Switch the stale-entry filter to `<= server_ms`** (Finding 9) and **delete the rescue pass**
   once 1–2 hold (Finding 8).
4. **Drain unconditionally** (remove the 4096 cap, Finding 10); hoist/verify `cleanup` (Finding 11).
5. Then the Pass-1 items (`walk_action` → ToDoQueue, `nextAction` → `Earliest*Time`), which become
   simpler once the queue/execute are exact.

## Reference index (Pass 2)

| Behavior | C++ 772 ref | Rust | Status |
|---|---|---|---|
| Priority queue structure | `containers.hh:150–227` (key-only binary heap) | `todo_queue.rs` (`BinaryHeap` + `sequence` + harness ties) | **Finding 6** |
| Equal-key drain order | structural sift order | FIFO `sequence` / pinned `harness_*_tie` | **Finding 6** |
| Execute step | `cract.cc:783` `while(true)` zero-delay loop | `run_monster_todo_execute` (one action/pop) | **Finding 7** |
| Delay computation | `cract.cc:906` `CalculateDelay` (`Earliest*Time`) | scattered (`earliest_attack_ms`, `next_wakeup`) | **Finding 7/Pass-1 #2** |
| Re-insertion | `ToDoStart`/`Execute` always `insert` | `schedule_creature_wakeup` insert | faithful |
| Stale-entry handling | pop + `NextWakeup > ServerMs` break | pop + `next_wakeup == execution_time` | **Finding 9** |
| Stranded-creature recovery | none (invariant holds) | `rescue_stalled_chase_monsters_772` per beat | **Finding 8** |
| Drain bound | unconditional (`crmain.cc:1144`) | `MAX_DRAINS_PER_BEAT = 4096` | **Finding 10** |
| Per-beat cleanup | not per-Execute | `cleanup()` per `process_creature_todo` | **Finding 11** |

*Pass 2 audit only — no code modified. Findings 6 and 7 are foundational: fix before further
monster-AI parity work.*

---

# Pass 3 — subsystem semantics, the other two clocks, and loop completeness

Pass 1 found the loop skeleton clean and two action-layer bleeds; Pass 2 found the queue/`Execute`
foundation. Pass 3 looks at what the four beat subsystems *actually do* — and finds that two of them
are wired to the wrong work, plus **two more wall-clock clocks** driving creature behavior inside the
beat (respawn, on top of Pass-1's `walk_action`). The pattern across all three passes is now explicit:
**CipSoft drives everything off `ServerMilliseconds` + the subsystem counters; the Rust 772 path keeps
sprinkling `Instant::now()` wall-clock timers into the beat.**

Reference: `main.cc:318–440` (`AdvanceGame` subsystem block), `crskill.cc` (`ProcessSkills`
timer-skills), `crmain.cc` (`ProcessCreatures`, `ProcessMonsterhomes`, `ProcessConnections`).
Rust: `game_world_tick.rs::advance_beat_772`, `subsystem_counters_772.rs`,
`creature_think.rs::process_creatures_772`, `spawn_lifecycle.rs::poll_spawn_respawns`,
`player_ping.rs`, `spell.rs`.

## Finding 12. `fired.skills` runs item decay; CipSoft `ProcessSkills` ticks creature timer-skills — HIGH

CipSoft's `SkillTimeCounter` (threshold 1250) fires `ProcessSkills`, which ticks the per-creature
**timer-skills** (`crskill.cc`): `TSkillPoison` / `TSkillBurning` / `TSkillEnergy` (damage-over-time),
`TSkillGoStrength` (haste/speed buff expiry), `TSkillLight`, `TSkillIllusion`, regeneration, and
player skill-advance timers — i.e. all the ~1 Hz creature effects.

`advance_beat_772` wires that slot to **item decay**:

```rust
if fired.skills {
    let _ = self.decay.tick(self.tick_counter);   // ITEM decay — wrong subsystem
}
```

Two distinct problems:
- **Conditions/skills don't tick at all on 772.** `docs/PROTOCOL_VERSIONING.md` (§ conditions)
  confirms the port has "**merge rules only — ticks are not yet implemented**." So fire/poison/energy
  DoT never decrements, haste/`GoStrength` never expires, and regeneration never runs on the 772
  logical clock — and the one subsystem that is *supposed* to drive them is busy doing item decay.
- **Item decay is in the wrong subsystem.** In CipSoft item/object decay is part of the cron/object
  system, not `ProcessSkills`. Wiring decay onto `SkillTimeCounter` (and keying it off the 1098
  50 ms `tick_counter` — see Pass-1 Finding 4) is a TFS-shaped graft, not the 772 model.

**Impact:** this is foundational for combat and status effects — a 772 monster's poison/fire should
tick once per `ProcessSkills` round; right now nothing does. **Recommendation:** implement
`ProcessSkills` as the creature timer-skill tick on `fired.skills` (logical round cadence), and move
item decay to its own cron/object schedule (or the `fired.cron` slot), off the 50 ms `tick_counter`.

## Finding 13. Monster respawn (`poll_spawn_respawns`) runs on wall-clock `Instant`, not logical time — HIGH

`poll_spawn_respawns` is invoked from `run_other_subsystems` (so, from the beat) but schedules
entirely on the **wall clock**:

```rust
pub fn poll_spawn_respawns(&mut self, now: Instant) {       // Instant, not server_ms
    if !self.spawns.should_run_check(now) { return; }
    let indices = self.spawns.due_slot_indices(now);
    self.spawns.mark_checked(now);
    // ... stall_respawn(slot_index, now) ...
}
```

CipSoft respawn is `ProcessMonsterhomes` on the `Other` (1 Hz) subsystem, with respawn delay counted
in **logical rounds / `ServerMilliseconds`**, not wall time. So this is the **third** wall-clock clock
inside the beat loop (with Pass-1's `walk_action_due` and `next_action_until`): it diverges from the
oracle under any lag/catch-up, can't be driven deterministically by the sim harness (which sets
`server_ms` directly), and means respawn timing is measured on a different clock than every other
creature event. **Recommendation:** drive respawn off `server_ms` / a logical round counter, checked
in the `Other` subsystem, like `ProcessMonsterhomes`.

## Finding 14. The `Other` and `cron` subsystems are largely unimplemented — MEDIUM (loop completeness)

CipSoft's `OtherTimeCounter` (1000) block does a lot (`main.cc:344–438`): `RoundNr += 1`,
`ProcessConnections` (idle-timeout/kick **and** ping), `ProcessMonsterhomes`, `ProcessMonsterRaids`,
`ProcessCommunicationControl`, reader/writer thread replies, `ProcessCommand`, ambiente/day-night
light broadcast (`SendAmbiente`), `NetLoadCheck`, and minute-boundary work (player-list refresh,
`SavePlayerData`, kill-statistics, reboot warnings). `CronTimeCounter` (1500) fires
`ProcessCronSystem`.

Rust `run_other_subsystems` does only: `poll_spawn_respawns` (≈ Monsterhomes), `lua_gc_step`,
`tick_player_pings` (≈ the ping half of `ProcessConnections`). `fired.cron` is an explicit no-op
(`"772 cron subsystem tick — no cron engine yet"`). Missing on the 772 loop: **idle-timeout kick**
(the other half of `ProcessConnections` — players are never disconnected for inactivity),
monster raids, **ambiente/day-night lighting**, periodic autosave, `RoundNr` (and everything keyed off
it). These are 772 loop responsibilities, not 1098 bleed, but they're gaps the foundation needs.
**Recommendation:** track them explicitly; at minimum wire idle-timeout and the ambiente light cycle,
which are observable to clients.

## Finding 15. The 1098 `nextAction` gate also governs 772 spells and uses — reinforces Pass-1 Finding 2

Pass 1 flagged `next_action_until` on movement. It's broader than that: the same 1098 instant gates
**spell casting** (`spell.rs::can_cast_instant` → `SpellFailReason::NextAction`) and item use/attack
(via the `handle_game_packet` gate). CipSoft 772 governs each of these with an **independent**
`Earliest*Time` evaluated in `CalculateDelay` (`cract.cc:906`): `EarliestSpellTime` for spells,
`EarliestMultiuseTime` for two-object use, `max(EarliestAttackTime, EarliestSpellTime)` for attack —
all against `ServerMilliseconds`. So on 772 the post-walk lockout shouldn't block a spell/use the way
the unified `nextAction` does. **Recommendation:** when porting the `Earliest*Time` model, replace the
`nextAction` checks in `spell.rs` and the packet gate together, not just the walk site.

## Finding 16. Subsystem-vs-clock ordering — verified faithful (no action)

`advance_beat_772` runs the subsystem handlers **before** `server_ms += delay` and the
`drain_todo_queue`, so `process_creatures_772` / decay / other see the *old* `server_ms` — matching
`AdvanceGame`, where `ProcessCreatures` etc. run before `MoveCreatures` does `ServerMilliseconds +=
Delay`. Correct. (The remaining ordering subtlety — that CipSoft coalesces `NumBeats·Beat` into one
call while Rust steps per beat — is Pass-1 Finding 3.)

## Cross-cutting observation: three wall-clocks in a logical-clock loop

The 772 beat is supposed to have exactly one clock (`ServerMilliseconds`) and one scheduler
(`ToDoQueue`). After three passes, the Rust 772 loop actually runs **four** time sources:

| Clock | Drives | Should be | Finding |
|---|---|---|---|
| `server_ms` / `ToDoQueue` | creature walk/AI ToDo | — (correct) | — |
| `walk_action_due` (`Instant`) | player walk-then-act | `ToDoQueue` | Pass-1 #1 |
| `next_action_until` (`Instant`) | player action lockout | `Earliest*Time` on `server_ms` | Pass-1 #2 / #15 |
| spawn `Instant` (`should_run_check`) | monster respawn | logical round | #13 |
| `tick_counter` (50 ms proxy) | item decay / lua gc cadence | logical subsystem | Pass-1 #4 / #12 |

Every wall-clock source is a place where the harness (which manipulates `server_ms` directly) and a
live server will disagree, and where lag/catch-up desyncs from the oracle. Collapsing them onto
`server_ms` is the same fix repeated, and it's the through-line of this whole audit.

## Recommended fix order (Pass 3 additions)

1. After the Pass-2 foundation (heap + `Execute`), **collapse the wall-clocks onto `server_ms`**:
   `walk_action` (Pass-1 #1), respawn (#13), then the `Earliest*Time` model replacing `nextAction`
   across walk + spell + use (Pass-1 #2 / #15).
2. **Implement `ProcessSkills`** as the creature timer-skill tick on `fired.skills`; move item decay
   off `tick_counter` (#12 / Pass-1 #4).
3. Fill the `Other`/`cron` gaps incrementally — idle-timeout and ambiente first (#14).

## Reference index (Pass 3)

| Behavior | C++ 772 ref | Rust | Status |
|---|---|---|---|
| Skills subsystem | `crskill.cc` `ProcessSkills` (timer-skills) | `fired.skills → decay.tick` (item decay) | **Finding 12** |
| Condition/skill ticks | `TSkillPoison/Burning/Energy/GoStrength/…` | not implemented (merge rules only) | **Finding 12** |
| Monster respawn | `ProcessMonsterhomes` (logical round) | `poll_spawn_respawns(Instant)` (wall-clock) | **Finding 13** |
| Other subsystem | `main.cc:344–438` (connections/raids/ambiente/save) | spawns + lua_gc + ping only | **Finding 14** |
| Cron subsystem | `ProcessCronSystem` | no-op stub | **Finding 14** |
| Spell/use action gate | `Earliest*Time` (`cract.cc:906`) | `next_action_until` (1098) | **Finding 15** |
| Subsystem vs clock order | `ProcessCreatures` before `MoveCreatures` | subsystems before `server_ms +=` | verified faithful |

*Pass 3 audit only — no code modified. Net: the loop and subsystem **scaffolding** is right, but
two subsystems are wired to the wrong work (#12, #14) and creature timing still rides three
wall-clocks that belong on `server_ms` (#13 + Pass-1 #1/#2). Combined with the Pass-2 queue/`Execute`
foundation, these are the correctness prerequisites the monster AI sits on top of.*

---

# Pass 4 — the `+1` re-insertion clamp, same-beat re-entry, and `CalculateDelay` gaps

Final pass, into the finest-grained scheduler detail: the exact `NextWakeup` offset on re-insertion.
This is small in code but large in behavior — it decides whether a re-planned action runs **this
beat** or **next beat**, and it interacts with every Pass-2/3 finding (drain order, the 4096 cap, the
rescue pass).

Reference: `cract.cc:1010–1024` (`ToDoStart`), `cract.cc:906–951` (`CalculateDelay`),
`cract.cc:1400–1535` (`NotifyGo` / `EarliestWalkTime` quantize), `crmain.cc:1142` (`MoveCreatures`).
Rust: `creature_todo.rs::todo_start_from_action` / `creature_todo_yield` / `todo_attack_delay_ms`,
`walk/mod.rs::todo_start_go_delay` / `schedule_immediate_todo_wakeup`.

## Finding 17. `ToDoYield` re-inserts at `server_ms + 0`; CipSoft clamps to `server_ms + 1` → same-beat re-entry — HIGH

CipSoft `ToDoStart` (`cract.cc:1015–1023`) **always** schedules at least one ms in the future:

```cpp
uint32 Delay = this->CalculateDelay();
if(Delay < 1){ Delay = 1; }                       // <-- minimum +1
uint32 NextWakeup = ServerMilliseconds + Delay;
ToDoQueue.insert(NextWakeup, this->ID);
```

`MoveCreatures` drains `while(top.Key <= ServerMilliseconds)` *after* `ServerMilliseconds += Delay`.
So a re-insertion at `ServerMilliseconds + 1` is **strictly greater** than the current
`ServerMilliseconds` and **cannot** be re-drained in the same beat — it lands on the next beat
(`+200`). This `+1` clamp is the engine's anti-re-entrancy guarantee: a creature that idles/yields at
beat N acts at beat N+1, never twice in beat N.

The Rust path is inconsistent about this:

```rust
// schedule_immediate_todo_wakeup — CORRECT (+1)
self.schedule_creature_wakeup(cid, self.server_ms.saturating_add(1), …);

// todo_start_go_delay — CORRECT (clamps ≥1)
let delay = calc_delay.max(1);
self.todo_start_from_action(cid, delay, …);

// todo_start_from_action — the leak: delay 0 schedules AT server_ms
pub(crate) fn todo_start_from_action(&mut self, cid, delay_ms, tie) {
    if delay_ms == 0 {
        self.schedule_creature_wakeup(cid, self.server_ms, tie);   // +0, NOT +1
    } else {
        self.schedule_creature_wakeup(cid, self.server_ms.saturating_add(delay_ms), tie);
    }
}

// creature_todo_yield — deliberately passes 0 with a comment that contradicts ToDoStart
self.todo_start_from_action(cid, 0, WakeupTiePolicy::HarnessAppearIdle);
// "C++ `ToDoWait(0)` — wakeup at `ServerMilliseconds`, not +1 (`cract.cc:1008`)."
```

The comment cites `:1008` (inside `ToDoWait`, which sets `Wait.Time = ServerMilliseconds + 0`), but
misses that `ToDoStart` at `:1015` then clamps the *resulting* `Delay` to `1`. `ToDoWait(0)` makes
`CalculateDelay` return 0; `ToDoStart` turns that 0 into `NextWakeup = ServerMilliseconds + 1`. **The
yield wakes next beat, not this beat.**

**Effect (concrete):** `drain_todo_queue` captures `due_limit = server_ms` and loops
`while peek.execution_time <= due_limit`. A yield re-inserted at `server_ms` satisfies
`server_ms <= server_ms`, so it is **popped again inside the same drain pass** — and `process_creature_todo`'s
validity check passes (the yield set `next_wakeup = server_ms`). So the Rust monster can re-run
`IdleStimulus` (and its RNG draws) repeatedly within one beat, where CipSoft would defer to the next
beat. This:
- changes idle/AI **timing** by up to a full beat (200 ms) per yield,
- changes **RNG draw order** (extra same-beat idle passes consume the glibc stream out of phase),
- changes **drain interleaving** with other creatures' same-`server_ms` entries (feeds Finding 6),
- and is a plausible driver of the 4096-drain cap (Finding 10) and the stall-rescue pass (Finding 8):
  a `+0` re-entry loop is exactly what a per-beat drain cap and a "stranded creature" safety net would
  be papering over.

**Recommendation:** make `todo_start_from_action` clamp like `ToDoStart` — `delay_ms.max(1)`,
schedule at `server_ms + max(1, delay)` unconditionally — and delete the `delay_ms == 0` special
case and the contradicting comment in `creature_todo_yield`. This is a one-line fix with outsized
correctness impact; re-validate the harness afterward (some pinned ties may shift because the
re-entry artifact disappears).

## Finding 18. `todo_attack_delay_ms` hardcodes `EarliestSpellTime = 0` — MEDIUM

CipSoft `CalculateDelay` for `TDAttack` (`cract.cc:933–940`):

```cpp
uint32 EarliestAttackTime = this->Combat.EarliestAttackTime;
if(EarliestAttackTime < this->EarliestSpellTime){
    EarliestAttackTime = this->EarliestSpellTime;     // attack waits on the later of the two
}
if(EarliestAttackTime > ServerMilliseconds){ Delay = EarliestAttackTime - ServerMilliseconds; }
```

The Rust port stubs the spell side to zero:

```rust
pub(crate) fn todo_attack_delay_ms(&self, cid) -> u64 {
    let earliest_spell_ms = 0u64;                      // <-- stub
    base.earliest_attack_ms.max(earliest_spell_ms).saturating_sub(self.server_ms)
}
```

So a `TDAttack` whose `EarliestSpellTime` is in the future (the monster just cast and the spell
gate should also hold the melee swing) computes too small a delay and the attack fires earlier than
CipSoft. With the casting loop already diverging (Monster-AI Pass-1 Finding 2) this compounds attack
cadence error. **Recommendation:** track `earliest_spell_server_ms` on the creature and feed it here,
mirroring the `max(EarliestAttackTime, EarliestSpellTime)` rule.

## Finding 19. `NotifyGo` step quantize to `Beat` — confirms Monster-AI audit, restated for the loop layer

For completeness in this scheduler audit: `NotifyGo` (`cract.cc:1531–1534`) sets
`EarliestWalkTime = ServerMilliseconds + ceil(Delay / Beat) * Beat` — walk-step delay quantized to a
multiple of **`Beat` (200 ms)**. `CalculateDelay`(`TDGo`) then yields that beat-multiple. The Rust
quantizer uses `step_beat_ms = 50` (`walk/walk_timing.rs`), already filed in
`MONSTER_AI_772_AUDIT.md` (Pass-2 "step-duration quantizer — latent, observably masked"): because
the live drain only fires on 200 ms boundaries, `ceil(V/50)*50` and `ceil(V/200)*200` collapse to the
same fire time — **except in the harness**, where `server_ms` can take non-200 values and the two
disagree. No new action here; flagged so the loop audit and AI audit agree on the authority
(`tibia-game-master` mechanics, not the TVP wire animation grid).

## Verified clean this pass (no action)

- `schedule_immediate_todo_wakeup` and the `todo_start_go_delay` `calc_delay.max(1)` path correctly
  honor the `+1` minimum — only the `delay==0` `todo_start_from_action` branch (yield) violates it.
- `CalculateDelay` `TDGo` / `TDWait` clamp-to-`EarliestWalkTime` and `TDUse` `EarliestMultiuseTime`
  branches map to the Rust `earliest_walk_server_ms` / multiuse paths (modulo Finding 18's spell gap
  and the Pass-1 `nextAction`/`Earliest*Time` swap).

## Reference index (Pass 4)

| Behavior | C++ 772 ref | Rust | Status |
|---|---|---|---|
| Re-insert min delay | `ToDoStart` `if(Delay<1)Delay=1` (`cract.cc:1016`) | `todo_start_from_action(0) → server_ms+0` | **Finding 17** |
| Yield wakeup | `ToDoYield`→`ToDoWait(0)`→`ToDoStart` (next beat) | `creature_todo_yield` (+0, same beat) | **Finding 17** |
| Attack delay | `max(EarliestAttackTime, EarliestSpellTime)` (`cract.cc:933`) | `earliest_attack_ms.max(0)` | **Finding 18** |
| Walk-step quantize | `ceil(Delay/Beat)*Beat` (`cract.cc:1533`) | `step_beat_ms = 50` | **Finding 19** (= AI audit) |
| `+1` on go/immediate paths | `ToDoStart` clamp | `schedule_immediate_todo_wakeup` / `.max(1)` | verified faithful |

*Pass 4 audit only — no code modified. Finding 17 (the `+1` clamp / same-beat re-entry) is the
highest-leverage single line in the whole scheduler: it likely underlies the symptoms patched by the
drain cap (#10) and the stall-rescue (#8), and it perturbs RNG draw order on every yield. Fix it
alongside the Pass-2 heap/`Execute` foundation.*

---

# Audit summary (4 passes)

The 772 loop **scaffolding is faithful** — loop selection, `BeatEndOnly` flush, `AdvanceGame`
subsystem cadence, `MoveCreatures` shape, and the era-split of the 1098 reactive scheduler are all
correct. The defects cluster in three layers, foundation-first:

1. **Scheduler core (Pass 2 + Pass 4):** the `ToDoQueue` equal-key order doesn't match CipSoft's
   structural heap (#6, papered over by hardcoded harness ties), `Execute` isn't atomic over
   zero-delay entries (#7), and the `+1` re-insertion clamp is violated by yield (#17). These change
   cross-creature execution and RNG-draw order on essentially every multi-creature beat — which is
   exactly why "the AI passes the sim but isn't right." Symptoms: the stall-rescue pass (#8), the
   `==` validity filter (#9), the 4096 drain cap (#10).
2. **Three wall-clocks where there should be one logical clock:** `walk_action_due` (#1),
   `next_action_until` / the `nextAction` gate across walk+spell+use (#2, #15), and monster respawn
   (#13) — all on `Instant`, all should be on `server_ms`. Plus the 50 ms `tick_counter` proxy (#4).
3. **Subsystem work mis-wired / missing:** `ProcessSkills` runs item decay instead of creature
   timer-skills (conditions/haste/regen never tick) (#12); the `Other`/`cron` subsystems are largely
   stubs (#14).

Recommended global order: **(a)** port CipSoft's `priority_queue` verbatim + drop the tie hacks
(#6); **(b)** make `Execute` the synchronous zero-delay loop (#7) and fix the `+1` clamp (#17), then
delete the rescue/cap/`==`-filter band-aids (#8/#9/#10); **(c)** collapse the wall-clocks onto
`server_ms` (#1/#2/#13/#15); **(d)** implement `ProcessSkills` condition ticks (#12) and fill the
`Other`/`cron` gaps (#14). Only after (a)–(b) will the harness ties be removable, which is the
clearest single signal that the foundation is finally correct.

---

# Pass 5 — first-ToDo arming + harness state in the production scheduler

A final sweep over creature *creation/appear* scheduling and the test-vs-production boundary. It
confirms Finding 17's blast radius and surfaces the deepest structural issue: **the production
scheduler branches on harness-only state, so the parity that "passes" is validated against a
scheduler configuration the live server never runs.**

Reference: `crnonpl.cc:2050–2112` (`TMonster::TMonster`), `:3196` (`ChallengeMonster`),
`cract.cc:1026` (`ToDoYield`). Rust: `monster_events.rs::monster_on_creature_appear_self`,
`idle_stimulus.rs::request_idle_stimulus`, `walk/mod.rs::todo_start_go_delay`,
`todo_queue.rs` tie policies, `game_world.rs` harness fields.

## Finding 20. First-ToDo arming is a `ToDoYield` — so Finding 17 affects every spawn — HIGH (amplifier)

`TMonster::TMonster` ends with `this->ToDoYield()` (`crnonpl.cc:2112`); `ChallengeMonster` and
`ConvinceMonster` paths do the same. So a freshly created monster's **first** schedule is
`ToDoYield → ToDoWait(0) → ToDoStart → NextWakeup = ServerMilliseconds + 1` — its first
`IdleStimulus` runs on the **next** beat, never the beat it spawned in. The Rust appear path
(`monster_on_creature_appear_self → request_idle_stimulus → creature_todo_yield`) mirrors the
*structure* but routes through the `+0` `todo_start_from_action` (Finding 17), so the first idle is
eligible **this** beat. Net: the `+1` clamp bug isn't an edge case — it governs the spawn-to-first-
action timing of every monster in the world, and every mid-life yield on top. Fixing Finding 17 fixes
this automatically.

## Finding 21. Harness-only state drives the production scheduler — CRITICAL (parity-validity)

`GameWorld` carries harness fields — `sim_harness_wall_ms`, `sim_harness_segment_ms`,
`batch_appear_defer_idle`, `harness_real_map` — and creatures carry `harness_defer_appear_idle`,
`harness_spawn_order`. These are **read by the production scheduling code**, not just tests:

- `todo_start_go_delay` computes the `TDGo` delay from `sim_harness_segment_ms` / `sim_harness_wall_ms`
  when set, **bypassing** the real `NotifyGo` `ceil(Delay/Beat)*Beat` quantize (Finding 19):
  ```rust
  let mut calc_delay = if !first_step {
      if let Some(segment) = self.sim_harness_segment_ms { segment }      // harness value
      else if earliest > server_ms { earliest - server_ms }
      else { self.todo_go_beat_delay_ms(cid) }                            // real path
  } else if earliest > server_ms { earliest - server_ms }
    else if let (Some(wall), Some(segment)) = (self.sim_harness_wall_ms, self.sim_harness_segment_ms) { … }
  ```
- `schedule_creature_wakeup` picks its equal-key tie from `harness_go_wakeup_tie_policy(cid)` →
  `harness_spawn_order` + `harness_real_map` (the Finding 6 maps).
- `request_idle_stimulus` / `run_monster_todo_execute` gate the first idle on
  `harness_defer_appear_idle` + `HARNESS_APPEAR_IDLE_DEFER_MS`.

**Why this is the deepest finding:** the harness sets `sim_harness_wall_ms`/`segment_ms` to scenario
`advance_ms` values and clamps how `server_ms` advances, so the passing parity tests exercise a
**substituted** delay/tie path — not the production `NotifyGo`/`CalculateDelay`/heap path. So a green
harness does **not** prove the live scheduler matches the oracle; it proves the live scheduler matches
the oracle *when fed harness hints*. This is the same root cause as Findings 6/17/19: the scheduler
can't reproduce the oracle on its own, so per-scenario hints were threaded through it. Once 6
(verbatim heap), 7 (atomic `Execute`), 17 (`+1`), and 19 (`Beat` quantize) are correct, **all** the
harness fields should be removable — and their removal is the acceptance test for the foundation.

**Recommendation:** treat full removal of `sim_harness_*` / `harness_*` from `GameWorld`, the
creature structs, and every scheduling function as the definition of done. The harness should drive
the engine only through the same inputs production uses (packets, `advance_beat_772(beat)`, real
spawns), asserting on outputs — never by injecting scheduler internals.

## Reference index (Pass 5)

| Behavior | C++ 772 ref | Rust | Status |
|---|---|---|---|
| First-ToDo arming | `TMonster::TMonster` → `ToDoYield` (`crnonpl.cc:2112`) | appear → `request_idle_stimulus` → yield (`+0`) | **Finding 20** (= #17) |
| Go-delay source | `NotifyGo`/`CalculateDelay` only | `sim_harness_segment_ms`/`wall_ms` override | **Finding 21** |
| Equal-key tie | structural heap | `harness_spawn_order` + `harness_real_map` maps | **Finding 21** (= #6) |
| First-idle gate | `ToDoYield` next beat | `harness_defer_appear_idle` + defer-ms | **Finding 21** |

*Pass 5 audit only — no code modified. Findings 20/21 close the loop: the scheduler defects (6/7/17/19)
forced harness hints (21) into production code, and the spawn path (20) means the `+1` bug is
universal. Removing the harness fields is the single clearest proof the foundation is correct.*

---

# Phased Implementation Plan

Foundation-first. Each phase is independently shippable, ends green (`rtk cargo test -p tfs-rust-core`
+ the chase-kite sim), and is sequenced so later phases delete the band-aids earlier ones expose. Do
**not** start a phase before the prior one is green — the whole point is that each removes a class of
hack the next depends on being gone.

## Phase 0 — Characterization & guardrails (no behavior change)

- Snapshot current chase-kite sim outputs (golden logs) and the full 772 test set as the regression
  baseline. These will legitimately *change* in Phases 1–2; capture them now so the diffs are
  reviewable, not surprising.
- Add a focused unit harness for the queue alone (insert/pop/equal-key) that asserts against the
  **CipSoft `priority_queue` algorithm** (port the C++ as an oracle in the test), independent of any
  monster AI. This is the acceptance test for Phase 1.
- Inventory every read of `sim_harness_*` / `harness_*` (Finding 21) so Phase 5's removal is mechanical.
- Deliverable: baselines + queue oracle test (red until Phase 1). No `src/` changes.

## Phase 1 — Port CipSoft `priority_queue` verbatim (Finding 6)

- Replace `BinaryHeap<Reverse<ToDoEntry>>` + `sequence` with a faithful array-backed binary heap:
  insert sift-up breaking on `Parent.Key <= Current.Key`, `deleteMin` sift-down with strict
  left-child bias (`containers.hh:162–222`). Key = `execution_time` only; data = `CreatureId`.
- Delete the `sequence` field and **all** `harness_*_tie` maps + `WakeupTiePolicy` (keep a thin
  insertion path; structural order replaces them).
- Expect harness drain-order tests to shift; re-bless against the oracle (Phase 0 queue test must pass
  first). If a scenario only passes *with* a tie map, that's a real ordering bug to chase, not re-pin.
- Deliverable: queue oracle test green; harness ties gone. Risk: high test churn — contained by Phase 0.

## Phase 2 — Make `Execute` atomic + fix the `+1` clamp (Findings 7, 17, 20)

- Rewrite `process_creature_todo` / `run_monster_todo_execute` as the CipSoft `Execute` `while(true)`
  loop: drain consecutive `CalculateDelay()==0` entries in one heap visit; on first `Delay>0`,
  `NextWakeup = server_ms + Delay`, insert, break; on empty, `IdleStimulus` + break.
- Change `todo_start_from_action` to schedule at `server_ms + delay.max(1)` unconditionally; delete the
  `delay==0` branch and the contradicting `creature_todo_yield` comment. This makes spawn/yield land
  next-beat (Finding 20).
- Switch the drain validity filter to `next_wakeup <= server_ms` (Finding 9) and drain unconditionally
  (remove `MAX_DRAINS_PER_BEAT`, Finding 10) — both are now safe because `+1` prevents same-beat
  re-entry.
- Deliverable: no same-beat re-entry; multi-entry zero-delay chains atomic. Re-bless sim logs.

## Phase 3 — Delete the band-aids (Findings 8, and confirm 9/10)

- Remove `rescue_stalled_chase_monsters_772`. Run the full sim + soak; any creature that now stalls is
  a real arming gap exposed by Phases 1–2 — fix at the `Execute`/`ToDoStart` site, not with a rescue.
- Confirm the cap/filter removals from Phase 2 hold under a multi-monster soak.
- Deliverable: rescue pass gone, no stalls. This is the first strong signal the core is self-consistent.

## Phase 4 — Collapse the wall-clocks onto `server_ms` (Findings 1, 2, 13, 15; Pass-1 #4)

- `walk_action` → enqueue as a `ToDo`/wakeup on `server_ms` under `beat_driven_loop` (drop
  `walk_action_due`).
- Replace the `nextAction` model with per-action `Earliest*Time` on `server_ms`, swapping the
  `handle_game_packet` gate **and** `spell.rs::can_cast_instant` together (Findings 2/15); wire
  `EarliestSpellTime` into `todo_attack_delay_ms` (Finding 18).
- Move monster respawn (`poll_spawn_respawns`) to a logical round counter in the `Other` subsystem
  (Finding 13).
- Drop the `delay_ms / 50` `tick_counter` proxy from `advance_beat_772` (Pass-1 #4).
- Deliverable: one clock (`server_ms`) drives all creature timing; harness `wall_ms`/`segment_ms` no
  longer needed for action timing.

## Phase 5 — Remove harness state from production (Finding 21) — the acceptance gate

- Delete `sim_harness_wall_ms`, `sim_harness_segment_ms`, `batch_appear_defer_idle`, `harness_real_map`,
  `harness_defer_appear_idle`, `harness_spawn_order` from `GameWorld`/creature structs and every
  scheduling function. Rework the harness to drive only via packets / `advance_beat_772` / real spawns,
  asserting on outputs.
- If anything regresses here, the foundation isn't done — return to the relevant phase. **Green sim
  with zero harness fields = foundation correct.**

## Phase 6 — Loop completeness & subsystem semantics (Findings 3, 12, 14, 19)

- Coalesce missed beats into `advance_beat_772(n·beat)` + add the `Delay < 1000` `MoveCreatures`
  lag-guard; fix the misleading comment (Finding 3).
- Implement `ProcessSkills` as the creature timer-skill tick on `fired.skills` (poison/fire/energy DoT,
  `GoStrength` expiry, regen); move item decay to its own cron/object schedule (Finding 12).
- Re-point the walk-step quantizer to `Beat` (200 ms) per `NotifyGo` (Finding 19) — now observable in
  the harness once `wall_ms` substitution is gone.
- Fill `Other`/`cron` gaps incrementally: idle-timeout kick + ambiente/day-night light first (Finding 14).
- Deliverable: subsystems do the right work at the right cadence.

## Sequencing rationale

1 → 2 → 3 are the load-bearing core and **must** be done in order (each removes a hack the next relies
on being absent). 4 → 5 collapse the clocks and prove it by deleting harness state. 6 is independent
content/cadence work that's clearer once the core is exact. The monster-AI parity items in
`MONSTER_AI_772_AUDIT.md` (push/kick era, RNG unification, casting loop) should be re-validated
**after** Phase 5 — several of its "feel" findings are downstream of the scheduler defects fixed here.

---

# Pass 6 — walk-execution internals (`Go`): drunk stagger + recovery throws

This pass reads the `TDGo` execution function (`TCreature::Go`) and the move-stimulus tail, since
those run *inside* the ToDo drain and draw RNG there. One clear new bleed; one path verified clean.

Reference: `cract.cc:379–447` (`Go`), `cract.cc:871–878` (`Execute` catch), `crmain.cc:920–951`
(`CreatureMoveStimulus`). Rust: `walk/mod.rs::try_drunk_walk_direction` + the `pop_dir` site,
`monster_events.rs::monster_combat_creature_move_stimulus`.

## Finding 22. Drunk-walk stagger is the 1098 algorithm on `thread_rng()` — HIGH (1098 + RNG bleed)

CipSoft `Go` staggers a drunk walker (`cract.cc:392–412`):

```cpp
int DrunkLevel = this->Skills[SKILL_DRUNKEN]->TimerValue();
if(DrunkLevel > 0 && this->Skills[SKILL_DRUNKEN]->Get() == 0){
    int StaggerChance = std::max<int>(7 - DrunkLevel, 1);   // level 1→1/6 … level 6→1/1
    if(rand() % StaggerChance == 0){                        // glibc draw #1
        DestX/DestY = OrigX/Y; switch(rand() % 4){ … }       // glibc draw #2 — random cardinal
        this->ToDoClear();
        this->ToDoTalk(TalkMode, NULL, "Hicks!", false);
        this->ToDoStart();                                   // re-schedule via the queue
    }
}
```

The Rust port implements the **1098** algorithm instead (`walk/mod.rs::try_drunk_walk_direction`,
the comment even cites `creature.cpp ~236–248`):

```rust
// TFS 1098 Creature::onWalk
let r = uniform_random(&mut thread_rng(), 0, 399);   // thread_rng — NOT glibc
if r / 4 > d { return None; }                        // 1098 curve: P(stagger) ≈ drunkenness/100
// else dir = rand % 4
```

Wrong on four axes (same shape as the monster-push defect in the AI audit):
1. **Algorithm:** 772 stagger probability is `1/max(7-DrunkLevel,1)` gated on `Skills[SKILL_DRUNKEN]->Get()==0`; Rust uses the 1098 `r/4 > drunkenness` curve. Different stagger frequency entirely.
2. **RNG stream:** `thread_rng()` (OS entropy, non-deterministic) instead of the glibc parity stream every other 772 draw uses — so it's non-reproducible run-to-run **and** doesn't advance the glibc stream, desyncing every subsequent `parity_*` draw for that creature (exactly the Finding-8 RNG-fragmentation class from the AI audit).
3. **Draw count/order:** 772 draws `rand()%StaggerChance` (always, when eligible) + `rand()%4` (on stagger); Rust draws `uniform_random(0,399)` + maybe `rand%4`. The per-step draw budget differs, so even a glibc-stream port would desync.
4. **ToDo restructure missing:** on stagger 772 does `ToDoClear` + `ToDoTalk("Hicks!")` + `ToDoStart` (re-plans through the queue); Rust just swaps the direction inline and broadcasts "Hicks!" — the queue isn't cleared/restarted, so the post-stagger scheduling differs.

Also note the input: 772 reads drunkenness from `Skills[SKILL_DRUNKEN]` — a **timer-skill** ticked by
`ProcessSkills` (Finding 12, not implemented), not a TFS `ConditionDrunk`. So the whole drunk
subsystem is on the 1098 model. **Recommendation:** port the 772 `Go` stagger exactly —
`max(7-level,1)` gate, glibc `parity_rand_mod`, the two-draw sequence, and the `ToDoClear/Talk/Start`
re-plan — and source drunkenness from the `ProcessSkills` timer-skill once Finding 12 lands. Until
then, at minimum move the draw onto the glibc stream so it stops poisoning RNG parity.

## Finding 23. Combat move-stimulus re-arm — verified faithful (no action)

`monster_combat_creature_move_stimulus` mirrors `TCreature::CreatureMoveStimulus` (`crmain.cc:920`)
accurately: gated on `ChaseMode == Close`, ToDo head is attack (`has_attack && !has_go`), `LockToDo`,
the `EarliestAttackTime <= ServerMilliseconds + 200` bail, and `distance <= 1` bail, then
`ToDoClear` + `Wait(200)` + re-arm attack. Event-driven from `monster_on_creature_move` (not the
per-beat scan). Correct — the only caveat is the `harness_real_map`/`harness_attacking_kite`
special-case, already covered by Finding 21.

## Also confirmed this pass

- `Go` is strictly **one tile** (`Distance > 1 → throw NOTACCESSIBLE`); multi-step paths are multiple
  `TDGo` entries each gated by `EarliestWalkTime`. This confirms Finding 7's scope: the atomic
  zero-delay batching in `Execute` applies to `Rotate`/`Talk`/ready-`Use`/ready-`Attack`, **not** to
  `Go` waypoints (which are beat-gated). No correction needed to Finding 7.
- Recovery is throw-type-dependent (`cract.cc:871`): `MOVENOTPOSSIBLE`/other → `ToDoYield`
  (retry next beat, no snapback for `MOVENOTPOSSIBLE`); `EXHAUSTED` → `ToDoWait(1000)` + `ToDoStart`.
  This matches the AI audit's Pass-2 Finding 7 framing; flagged here so the loop audit and AI audit
  agree on the recovery contract.

## Reference index (Pass 6)

| Behavior | C++ 772 ref | Rust | Status |
|---|---|---|---|
| Drunk-walk stagger | `Go` `rand()%max(7-level,1)` + `rand()%4`, glibc (`cract.cc:392`) | 1098 `r/4>drunk` on `thread_rng` | **Finding 22** |
| Drunkenness source | `Skills[SKILL_DRUNKEN]` timer-skill | `ConditionDrunk` + `drunkenness` field | **Finding 22** (= #12) |
| Combat move re-arm | `CreatureMoveStimulus` (`crmain.cc:920`) | `monster_combat_creature_move_stimulus` | verified faithful |
| `Go` single-step | `Distance>1 → NOTACCESSIBLE` | `TDGo` per step | confirms #7 scope |
| Blocked-step recovery | throw → `Yield`/`Wait(1000)` | `walk/mod.rs` recovery | = AI audit #7 |

*Pass 6 audit only — no code modified. Finding 22 adds drunk-walk to the wall of 1098/RNG bleeds
(joins monster-push and the `ai_rng`/`thread_rng` fragmentation from the AI audit). It belongs in
Phase 4 (RNG/clock unification) and Phase 6 (`ProcessSkills` for the drunkenness source).*

---

# Pass 7 — event fan-out ordering (move stimulus)

This pass follows the move-stimulus *dispatch* — when one creature moves, the order in which nearby
monsters receive `CreatureMoveStimulus` (and thus `ToDoClear`/re-arm/RNG-draw). Like the heap tie
(Finding 6), this is an ordering concern, but at the event-dispatch layer.

Reference: `crmain.cc:920` + `map.cpp` `AnnounceChangedObject` / sector iteration. Rust:
`monster_events.rs::monsters_witnessing_move` / `monster_dispatch_creature_move`,
`map/grid.rs::collect_spectators`.

## Finding 24. Move-stimulus fan-out is in SlotMap-key order, not spatial order — HIGH (ordering)

`monsters_witnessing_move` collects witnessing monsters from the chunk grid, then orders them:

```rust
if self.harness_real_map {
    ids.sort_by(|a, b| order(*b).cmp(&order(*a)));   // harness: by harness_spawn_order (Finding 21)
} else {
    ids.sort_by_key(|id| id.data().as_ffi());         // production: by SlotMap key = creation order
}
```

So in production, `monster_on_creature_move` fires on witnesses in **SlotMap-key (creature-creation)
order**. CipSoft dispatches `CreatureMoveStimulus` as part of the move announce, iterating creatures
in **spatial sector/coordinate order** (`map.cpp` sector walk), not creation order. Because each
witnessing monster's stimulus can `ToDoClear` + re-arm its attack/chase (Finding 23) and draw RNG,
the *order* of the fan-out determines:

- the order competing chasers re-plan and re-insert into the `ToDoQueue` (compounding Finding 6's
  equal-key heap order — two orderings, both non-spatial), and
- the glibc draw order for any stimulus-driven RNG.

SlotMap-key order is essentially arbitrary w.r.t. position (allocation order, with slot reuse), so it
diverges from the oracle's spatial walk for any multi-witness move. As with Findings 6 and 21, the
harness hides this by sorting on `harness_spawn_order` — so the pinned scenarios pass while the
production fan-out order is wrong.

**Recommendation:** dispatch `onCreatureMove`/stimulus in the same spatial order CipSoft uses — walk
the affected sectors/tiles in coordinate order (the `Map::getSpectators` / `AnnounceChangedObject`
traversal), not a `SlotMap`-key sort. Then drop the `harness_real_map` spawn-order sort (folds into
Finding 21's removal). This must agree with the Finding-6 heap order so re-arm → re-insert → drain is
coherent end to end.

## Also confirmed this pass

- `collect_spectators` (`grid.rs`) is an explicit chunk-overlap **superset** iterated `chunk_y` outer
  / `chunk_x` inner, with per-chunk `creatures` in insertion order — fine for packet fan-out (clients
  don't depend on order) but **not** a spatial per-tile order, which is why the AI fan-out then needs
  an explicit sort. The fix in Finding 24 should produce a per-tile coordinate order for the stimulus
  path specifically; the packet path can keep the cheap superset.
- Packet move broadcasts (`send_move_creature_spectator`, turn `0x6B`) iterate spectator *connections*
  and gate on `can_see_*` — order-independent and correct; no action.

## Reference index (Pass 7)

| Behavior | C++ 772 ref | Rust | Status |
|---|---|---|---|
| Move-stimulus fan-out order | spatial sector walk (`map.cpp` announce) | `sort_by_key(SlotMap key)` / `harness_spawn_order` | **Finding 24** |
| Spectator collection | `Map::getSpectators` superset | `collect_spectators` chunk superset | faithful (superset; order via #24) |
| Move packet fan-out | per-spectator `canSee` | conn iterate + `can_see_*` | faithful |

*Pass 7 audit only — no code modified. Finding 24 is the event-layer twin of Finding 6: two
non-spatial orderings (heap tie + stimulus fan-out) that the harness pins with `harness_spawn_order`.
Both must move to the oracle's deterministic order and must agree with each other. Add to Phase 1
(make the fan-out order spatial alongside the verbatim heap) so the harness ties can be removed
together.*

---

# Pass 8 — NPC scheduling (queue-driven in 772, think-driven in Rust)

This pass checks how NPCs are scheduled. In 772 NPCs are first-class `ToDoQueue` creatures, exactly
like monsters; the Rust path routes them through the 1 Hz `ProcessCreatures` think instead — and that
think is a stub.

Reference: `crnonpl.cc:1718` (`TNPC::IdleStimulus`), `:1100–1290` (behaviour talk → `ToDoWait`/
`ToDoTalk`/`ToDoStart`), `:1811` (`TNPC::CreatureMoveStimulus`), `:1980` (`ChangeNPCState`). Rust:
`creature_think.rs::npc_on_think` / `process_creatures_772`, `creature_todo.rs::creature_uses_todo_execute`.

## Finding 25. NPCs don't use the ToDoQueue — they run on the stubbed 1 Hz think — HIGH (architecture + incompleteness)

CipSoft NPCs are driven entirely through the `ToDoQueue`:

```cpp
// TNPC::IdleStimulus (crnonpl.cc:1718) — wander
this->ToDoGo(DestX, DestY, DestZ, true, INT_MAX);
this->ToDoWait(2000);
this->ToDoStart();
// behaviour talk (crnonpl.cc:1111/1286)
Npc->ToDoWait(TalkDelay);
Npc->ToDoTalk(TALK_SAY, NULL, Response, false);
Npc->ToDoStart();
```

So an NPC's `IdleStimulus` fires when its ToDo list drains (a queue wakeup), it wanders with a
`ToDoGo + ToDoWait(2000)` cadence, and talk responses are queued with per-character delays — all on
`ServerMilliseconds` via `MoveCreatures`, identical in mechanism to monsters.

The Rust 772 path does **not** schedule NPCs on the queue:

- `process_creatures_772` includes NPCs but calls `npc_on_think` on the **1 Hz `ProcessCreatures`
  bucket**, not the queue. CipSoft's `ProcessCreatures` does not drive NPC idle — the queue does.
- `npc_on_think` is a **stub**: `creature_on_think` (target/follow bookkeeping) plus a comment
  `// D.6: random step within master_radius, focus / turn-to-speaker` — no idle walk, no talk pacing.
- `creature_uses_todo_execute` returns `true` **only for monsters**, so even if an NPC had a
  `next_wakeup`, `process_creature_todo` would fall through to the 1098-style `on_walk` branch rather
  than the 772 `IdleStimulus`/ToDo execute path. NPCs never enter the `IdleStimulus → ToDoGo/Wait/
  Start` loop.

**Effect:** 772 NPCs don't wander on the correct cadence, their talk/response delays aren't
queue-paced, and `CreatureMoveStimulus` (turn-to-approaching-player, queue management) isn't wired
the 772 way. Functionally NPCs are mostly inert vs the oracle. Architecturally it's the same
think-vs-queue mismatch as the monster path would have if it weren't queue-driven — NPCs were left on
the 1098-shaped think.

**Recommendation:** treat NPCs as queue creatures: extend `creature_uses_todo_execute` (and the
`IdleStimulus`/`ToDoYield`/`request_idle_stimulus` arming) to NPCs, port `TNPC::IdleStimulus`
(wander `ToDoGo + ToDoWait(2000)`) and the behaviour-talk `ToDoWait/ToDoTalk/ToDoStart` pacing, and
drive NPC `onCreatureMove` (focus/turn, queue) from the stimulus rather than the 1 Hz think. This
depends on the Phase-1/2 queue + `Execute` foundation being correct first.

## Also confirmed this pass

- NPC `MovePossible` (`crnonpl.cc:1672`) is non-throwing (BANK && !UNPASS && not avoid) — simpler
  than the monster `MovePossible`/kick path; the NPC idle-walk port doesn't need the kick machinery.
- NPC recovery uses the same `ToDoWait(2000)+ToDoStart` re-arm on a blocked/failed idle — so once
  NPCs are queue-scheduled, the `+1` clamp (Finding 17) and atomic `Execute` (Finding 7) apply to
  them identically; no separate NPC scheduler is needed.

## Reference index (Pass 8)

| Behavior | C++ 772 ref | Rust | Status |
|---|---|---|---|
| NPC idle / wander | `TNPC::IdleStimulus` → `ToDoGo+Wait(2000)+Start` (`crnonpl.cc:1718`) | `npc_on_think` stub on 1 Hz think | **Finding 25** |
| NPC talk pacing | `ToDoWait(TalkDelay)+ToDoTalk+ToDoStart` | not queue-paced | **Finding 25** |
| NPC queue participation | full `ToDoQueue` creature | `creature_uses_todo_execute` monster-only | **Finding 25** |
| NPC move stimulus | `TNPC::CreatureMoveStimulus` (focus/queue) | not wired (1 Hz think) | **Finding 25** |

*Pass 8 audit only — no code modified. Finding 25 is an architecture+completeness gap: NPCs were left
on the 1098-shaped 1 Hz think while monsters moved to the queue. Fold the NPC queue port into Phase 6
(after the Phase-1/2 foundation), reusing the same `Execute`/`+1`/idle machinery as monsters.*

---

# Pass 9 — death/removal timing during the drain

This pass checks what happens when a creature dies mid-beat — the interaction between `Kill`/`Death`,
the `ToDoQueue`, and what other creatures see for the rest of that beat.

Reference: `cr.hh:567` (`Kill` → `Death`), `crmain.cc:879` (`TCreature::Death` sets `IsDead`),
`cract.cc:785` (`Execute` breaks on `IsDead`). Rust: `game_world_lifecycle.rs::apply_creature_death`
/ `remove_creature` / `release_creature` / `cleanup`.

## Finding 26. Death removes the creature immediately; CipSoft sets `IsDead` and flushes later — MEDIUM (verify)

CipSoft `Kill()` (`cr.hh:567`) sets HP to 0 and calls `Death()`, which only does:

```cpp
void TCreature::Death(void){ this->IsDead = true; this->LoggingOut = true; }   // crmain.cc:879
```

It does **not** delete or unlink the creature. The creature stays in the world with `IsDead = true`
for the remainder of the beat: `Execute` pops any stale `ToDoQueue` entry and breaks on `IsDead`
(`cract.cc:785`), and other creatures still resolve it (as a dying entity) until a later flush
deletes it (corpse creation in `~TMonster`). So within the same beat, observers see a consistent
"present-but-dead" snapshot; removal is deferred to a flush point.

The Rust path removes immediately:

```rust
// apply_creature_death → remove_creature(victim)  — immediate, mid-drain
self.remove_creature(victim);   // unregisters tile, drops SlotMap entry, removes summons, clears lookups
```

And the deferred machinery that would match CipSoft — `release_creature` →
`creatures_pending_release` → `cleanup` (cited as TFS `Game::ReleaseCreature`/`ToReleaseCreatures`) —
**exists but has zero callers for creatures**: a grep shows only `release_item` is ever called;
`release_creature` is dead code. So death bypasses the deferral entirely.

**Effect (to verify):** because removal is immediate, a creature that dies during creature A's
`Execute` is gone before creatures B, C… `Execute` later in the **same** beat. Versus CipSoft, where
B/C still observe the dead-flagged victim until the flush. This can change, within the killing beat:

- target/follow re-acquisition for other creatures that were targeting the victim (they see "gone"
  immediately vs "present, dead"),
- tile occupancy / `MovePossible` for movers crossing the victim's tile,
- summon teardown timing (Rust removes a dead master's summons instantly; CipSoft defers),
- the ordering of the victim's "disappear"/corpse packets relative to other same-beat events.

It also compounds Finding 11 (per-entry `cleanup`) — the two together mean entity lifetime within a
beat is more granular than the oracle's "all Executes run, then flush."

**Recommendation:** route creature death through a deferred path — mark the victim dead (an
`is_dead`/removed flag, `Execute`/AI skip it like CipSoft's `IsDead`) and remove it at a single
end-of-drain flush, wiring up the already-present `creatures_pending_release`/`cleanup` for creatures
(not just items). Then verify against the oracle whether same-beat observers see the dead-but-present
snapshot. If a full `IsDead` snapshot is too invasive, at minimum confirm immediate removal is
observationally equivalent for the shipped scenarios before relying on it.

## Also confirmed this pass

- The `ToDoQueue` holds `CreatureId` (not pointers), and the drain validity filter / `creatures.get`
  lookup safely skip a removed creature's stale entries — so immediate removal is *memory-safe*
  (unlike the C++ pointer model that forced deferral); the concern is purely **observable ordering**,
  not safety.
- `remove_creature` recurses into summons correctly (master death → summon removal), matching the
  `~TMonster`/summon-chain intent — only the *timing* (immediate vs flush) is in question.

## Reference index (Pass 9)

| Behavior | C++ 772 ref | Rust | Status |
|---|---|---|---|
| Death marking | `Death()` sets `IsDead`, keep in world (`crmain.cc:879`) | `apply_creature_death → remove_creature` immediate | **Finding 26** |
| Deferred release | flush dead creatures later | `creatures_pending_release` (dead code for creatures) | **Finding 26** |
| Stale entry of dead creature | pop + `IsDead` break | pop + `creatures.get()==None` skip | faithful (safe) |
| Summon teardown on master death | summon chain | `remove_creature` recursion | faithful (timing via #26) |

*Pass 9 audit only — no code modified. Finding 26 is an observable-ordering item (not a safety bug):
death is immediate where CipSoft defers to an end-of-beat flush, and the deferral machinery is present
but unused for creatures. Fold into Phase 2/3 (alongside the `Execute`/`cleanup` rework) so entity
lifetime within a beat matches the oracle.*

---

# Audit status after 9 passes

Nine passes have now covered: the loop skeleton and flush policy (Pass 1), the `ToDoQueue` structure
and `Execute` (Pass 2), subsystem semantics and the wall-clocks (Pass 3), the `+1` re-insertion clamp
and delay computation (Pass 4), first-ToDo arming and harness entanglement (Pass 5), walk-execution
internals/drunk (Pass 6), event fan-out ordering (Pass 7), NPC scheduling (Pass 8), and death/removal
timing (Pass 9). 26 findings total; the foundational set is **6** (heap order), **7** (atomic
`Execute`), **17** (`+1` clamp), **24** (fan-out order), and the **wall-clock collapse** (1/2/13) —
everything else is symptom, completeness, or cleanliness layered on those.

Confidence: the **scheduler/loop/ToDoQueue core** is now traced end-to-end against the CipSoft sources
and I do not expect further structural foundation findings — remaining risk is in **leaf behavior**
(individual `ToDo` action handlers: `Move`/`Trade`/`Use` cylinder semantics, exact `GetSpeed`/
`GoStrength` and skill-advance math, condition-tick formulas once `ProcessSkills` exists) and in
**content breadth** (cron events, raids, ambiente), which are better validated by golden-trace diffs
against the oracle than by more structural reading. The phased plan above remains the path; the new
Pass 6–9 findings slot in as noted (22→Phase 4/6, 24→Phase 1, 25→Phase 6, 26→Phase 2/3).


---

# Implementation log

## Phase 0 + Phase 1 — DONE (verbatim `priority_queue`, harness ties removed)

- **`todo_queue.rs` rewritten** as a verbatim port of CipSoft `priority_queue<uint32,uint32>`
  (`containers.hh:150–227`): 1-indexed array binary heap, key = `execution_time` only, **no
  secondary key**. `insert` sift-up (`Parent.Key <= Current.Key ⇒ stop`) and `delete_min` sift-down
  (strict left-child bias `Other.Key < Smallest.Key`) transcribed exactly. Removed `ToDoEntry.sequence`,
  `insert_with_tie`, `bump_sequence`, `WakeupTiePolicy`, and the `harness_go_step_tie` /
  `harness_appear_idle_tie` / `harness_go_step_tie_realmap_bowl` maps (Finding 6).
- **Phase 0 guardrail** added: an independent literal transcription of the CipSoft algorithm
  (`CipSoftOracle`) plus a differential test over 200 randomized insert/pop sequences with a small
  key space (frequent equal-key ties). Production queue and oracle agree on every pop, including
  structural ties (`equal_key_order_is_structural_not_fifo`: A,B,C@equal → pop A,C,B, not FIFO).
- **Call sites migrated**: `schedule_creature_wakeup(cid, execution_time)` and
  `todo_start_from_action(cid, delay)` dropped the `tie_policy` param across `walk/mod.rs`,
  `creature_todo.rs`, `idle_stimulus.rs`, `sim_harness.rs`. Deleted `harness_go_wakeup_tie_policy` /
  `harness_attack_wakeup_tie_policy`. Equal-key drain order is now structural everywhere.
- **Scope kept tight**: the `+1` re-insertion clamp (Finding 17) was intentionally **not** changed
  here — `todo_start_from_action(_, 0)` still schedules `+0`, with a comment marking it Phase 2. The
  move-stimulus fan-out order (Finding 24) is also deferred (it still uses the SlotMap-key sort; the
  `harness_real_map` branch in `monsters_witnessing_move` remains until Phase 5 harness removal).

### Verification

- `rtk cargo build -p tfs-rust-core` — clean (warnings only).
- `rtk cargo test -p tfs-rust-core -- todo_queue::tests` — **4/4 pass** (incl. CipSoft-oracle
  differential).
- Full `rtk cargo test -p tfs-rust-core` — **378–380 pass**; the harness drain-order/tie tests did
  **not** need re-blessing (structural order already matches), which is the Phase-1 success signal.

### ⚠️ Flaky tests (pre-existing, NOT caused by Phase 1)

The full-suite run surfaces a small, **non-deterministic** set of failures in
`idle_stimulus::tests` that varies run-to-run (observed 4 then 2 failing across consecutive runs):
`test_772_dist_target_flee_inline_chase_after_goal_wait`, `test_e4_cobra_poison_at_range`,
`test_e4_spell_delay_gate`, `test_772_attacking_close_chase_at_cheb11`. Established as flake, not
regression:

- Stashing Phase 0/1 and running on the **pre-change baseline** reproduces 3 of the 4 failures
  (`dist_target_flee`, `cobra_poison`, `spell_delay_gate`) — they pre-date this work.
- `test_772_attacking_close_chase_at_cheb11` **passes deterministically in isolation** but flakes in
  the parallel suite; `spell_delay_gate` reports different counts run-to-run (`got 1` vs `got 0`).
- Root cause is the process-global RNG interference documented in `MONSTER_AI_772_AUDIT.md` Finding 8
  (`thread_rng` / `libc::srand` shared across parallel tests). These tests assert on RNG-dependent
  idle/cast outcomes without a per-test seed, so parallel execution perturbs them.
- **Resolution path:** the RNG unification (audit Phase 4 / AI-audit Finding 8) will make these
  deterministic; until then they should be `#[serial]` or seeded. Not a Phase-1 blocker.

## Phase 2 — IN PROGRESS

Atomic `Execute` loop + `+1` re-insertion clamp; then switch the stale-entry filter to
`<= server_ms` and remove the `MAX_DRAINS_PER_BEAT` cap (Findings 7, 17, 20, 9, 10).
