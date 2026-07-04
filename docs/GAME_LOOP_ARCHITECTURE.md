# Game Loop Architecture

This document defines the threading model and game loop design for both supported eras.
**One binary, one beat engine** — selected by `clientVersion` in `config.lua`. Per-era
differences (beat size, think cadence, condition/skill tick interval, flush policy, walk
speed model) live in `MechanicsProfile` (+ `data/formulas/<v>.lua`) and `ProtocolCodec` —
never in a loop-selection branch.

> **Implementation status (2026-07-05)**
>
> | Section | Status |
> |---------|--------|
> | **§2 — Unified beat engine** | **Implemented.** `run_game_loop` is the single loop entry point for both eras. Phase 5 deleted the 1098 reactive loop (`run_game_loop_1098`); Phase 6 collapsed the `beat_driven_loop` flag; Phase 7 merged the last `*_772` loop alias into `run_game_loop`. |
> | **§3 — Per-era profile knobs** | **Implemented.** `MechanicsProfile` carries `beat_ms`, think cadence, tick interval, flush policy, `StepSpeedModel`. 772 = `LinearGo` + 200 ms beat + staggered ~1000 ms subsystems + beat-end flush; 1098 = `TfsLog` + 50 ms beat + 50 ms bucketed think + immediate-on-movement flush. |
> | **§4 — C++ reference index** | Both eras still cite their authoritative C++ sources. |
>
> See [`CODEBASE_AUDIT.md`](CODEBASE_AUDIT.md) for the full gap analysis and
> [`unified-beat-engine-phases.md`](../tasks/unified-beat-engine-phases.md) for the
> unification effort history.

---

## 1. Shared Threading Model

Both eras share the same hybrid threading model: **single-threaded game simulation** +
**asynchronous Tokio I/O**.

```
┌─────────────────────────────────┐           ┌─────────────────────────────────┐
│     Tokio Multi-Threaded I/O    │           │     Single-Threaded Game Loop   │
│         (tfs-rust-net)          │           │         (tfs-rust-core)         │
├─────────────────────────────────┤           ├─────────────────────────────────┤
│ - Accepts connections           │           │ - GameWorld (Creatures & Items) │
│ - Packet parsing & serialization│   mpsc    │ - Spatial grid map (Map)        │
│ - RSA/XTEA encryption           │  ───────> │ - Event dispatcher              │
│ - DB queries (SQLx async)       │           │ - Process player commands       │
│ - TCP Socket read/write         │           │ - Sequential execution          │
└─────────────────────────────────┘           └─────────────────────────────────┘
```

* **Game Thread (Single-Threaded):** Owns all game state including `GameWorld`, `SlotMap` entity
  storage, and `Map` grid. None of these types are `Send` or `Sync`.
* **I/O Threads (Tokio Tasks):** Handle concurrent network/DB work. Communication via lockless
  `mpsc` channels.

---

## 2. The Unified Beat Engine

Both eras run on a single beat-driven loop: `run_game_loop` in
`crates/tfs-rust-core/src/game_loop.rs`. There is no era fork in the loop body — every
per-era difference is read from `MechanicsProfile` at loop start and inside `advance_beat`.

### 2.1 C++ Reference Models

The unified engine reconciles two C++ architectures under one Rust implementation. The
*observable behavior* of each era is preserved; the *loop structure* is the CipSoft 7.72
beat loop, which is the more constrained of the two (1098's reactive Dispatcher is a
relaxation of beat quantization, recoverable by tuning `beat_ms` / cadence / flush policy
in the profile).

| Era | Authoritative C++ source | Loop shape |
|-----|--------------------------|------------|
| **772** | `tibia-game-master/src/main.cc` `LaunchGame` (477-492) + `AdvanceGame` (312-449) | Signal-driven beat loop + global `ToDoQueue` + `SendAll` |
| **1098** | `src/tasks.cpp` (Dispatcher) + `src/scheduler.cpp` + `src/game.cpp` `checkCreatures` / `checkCreatureWalk` | Dispatcher FIFO + per-event `steady_timer` + inline flush |

The 1098 reactive loop (`run_game_loop_1098`) was deleted in Phase 5; 1098 now runs on the
same beat engine with profile knobs that reproduce the 1098 feel (50 ms beat, 50 ms bucketed
think, immediate-on-movement flush). The 1098 sign-off against a live 10.98 client is the
Phase 9 gate.

### 2.2 Rust Implementation

File: `crates/tfs-rust-core/src/game_loop.rs`

```rust
pub async fn run_game_loop(
    mut world: GameWorld,
    mut cmd_rx: UnboundedReceiver<GameCommand>,
    out_registry: Option<OutRegistry>,
) -> anyhow::Result<()> {
    // Beat size comes from the profile — 200 ms (772) or 50 ms (1098).
    let beat_ms = u64::from(world.mechanics.profile.beat_ms.max(1));
    let mut beat_timer = interval(Duration::from_millis(beat_ms));
    beat_timer.set_missed_tick_behavior(MissedTickBehavior::Burst);
    let mut pending: VecDeque<GameCommand> = VecDeque::new();

    loop {
        tokio::select! {
            biased;

            // Branch 1: Network commands + DB callbacks.
            // C++ 772: SIGUSR1 → ReceiveData (drain all pending input).
            // C++ 1098: g_dispatcher.addTask → FIFO execute inline.
            cmd = recv_next_command(&mut cmd_rx, &mut pending) => {
                match dispatch_command(&mut world, cmd, &mut cmd_rx,
                                       &mut pending, &out_registry).await {
                    ControlFlow::Break(LoopExit::Shutdown) => {
                        flush_online_players_to_db(&world).await?;
                        break;
                    }
                    ControlFlow::Break(LoopExit::ChannelClosed) => break,
                    ControlFlow::Continue(()) => { /* drain rest of burst */ }
                }
            }

            // Branch 2: Beat timer fires.
            // C++ 772: SIGALRM → AdvanceGame(NumBeats * Beat).
            // C++ 1098: the 50 ms recurring checkCreatures / checkCreatureWalk timers.
            _ = beat_timer.tick() => {
                let mut beats = drain_burst_beats(&mut beat_timer);
                if beats == 0 { beats = 1; }
                world.advance_beat(beat_ms * beats);   // profile-driven cadence/flush inside
                while let Some(conn_id) = world.pending_idle_kick.pop() {
                    handle_player_disconnect(&mut world, conn_id, false, &out_registry);
                }
                flush_pending_outgoing(&mut world, &out_registry);   // SendAll
            }
        }
    }
    Ok(())
}
```

**Mapping to C++ (both eras):**

| Rust | 772 C++ equivalent | 1098 C++ equivalent |
|------|--------------------|---------------------|
| `cmd_rx` (unbounded mpsc) | `ReceiveData` drain on `SIGUSR1` | `g_dispatcher` task queue |
| `beat_timer` (interval) | POSIX `timer_t` at `Beat` → `SIGALRM` | `g_scheduler.addEvent(checkCreatures, 1000ms)` + per-creature `checkCreatureWalk` timers |
| `biased;` cmd-first ordering | `SIGUSR1` processed before `SIGALRM` in `sigwait` loop | Dispatcher FIFO: `playerMove` arriving first always runs before `checkCreatureWalk` |
| `advance_beat` | `AdvanceGame(NumBeats * Beat)` — staggered counters + `MoveCreatures` | `checkCreatures` bucket + `checkCreatureWalk` per-creature |
| `flush_pending_outgoing` | `SendAll()` — once per beat | inline per-handler socket writes (reproduced via profile flush policy) |
| `pending` VecDeque (Turn→Move coalescing) | N/A (Rust-specific OTC input coalescing) | N/A — Rust-specific optimisation |
| `drain_burst_beats` | `NumBeats = SigAlarmCounter` (multi-beat lag catch-up) | tick-overrun warning (45 ms / 50 ms) |

### 2.3 `advance_beat` — The Tick Pipeline

`GameWorld::advance_beat` (was `advance_beat_772`; the `_772` suffix is scheduled for
removal in Phase 8) runs the staggered subsystem counters, drains the global `ToDoQueue`,
and applies the profile-selected flush policy. The pipeline mirrors
`tibia-game-master/src/main.cc:312-449` `AdvanceGame`:

```c
// main.cc:312-449 (reference)
static void AdvanceGame(int Delay){
    CreatureTimeCounter += Delay;
    CronTimeCounter     += Delay;
    SkillTimeCounter    += Delay;
    OtherTimeCounter    += Delay;

    if(CreatureTimeCounter >= 1750){ CreatureTimeCounter -= 1000; ProcessCreatures(); }
    if(CronTimeCounter     >= 1500){ CronTimeCounter     -= 1000; ProcessCronSystem(); }
    if(SkillTimeCounter    >= 1250){ SkillTimeCounter    -= 1000; ProcessSkills(); }
    if(OtherTimeCounter    >= 1000){ OtherTimeCounter    -= 1000; RoundNr += 1;
                                     ProcessConnections(); /* ... light, spawns ... */ }
    if(Delay < 1000){ MoveCreatures(Delay); }   // lag guard
    SendAll();
}
```

**Key properties preserved:**

- **Staggered subsystems:** each subsystem has an independent counter with a different
  initial threshold (1750, 1500, 1250, 1000 ms); all reset by 1000 ms. This staggers their
  first execution across different beats to spread CPU load. The thresholds and reset are
  profile-driven so 1098 can adopt a non-staggered 50 ms cadence.
- **`MoveCreatures` skipped during lag:** if `Delay >= 1000` (5+ missed beats at 200 ms),
  creature movement is suppressed entirely.
- **Single `SendAll()` at end:** all output for all connections is flushed exactly once per
  beat — no intermediate flushes (772). 1098's immediate-on-movement flush is reproduced via
  the profile flush policy inside `advance_beat`, not by a separate loop branch.

### 2.4 `MoveCreatures` — The ToDoQueue Priority Heap

```c
// crmain.cc:1106-1122 (reference)
void MoveCreatures(int Delay){
    ServerMilliseconds += Delay;
    while(ToDoQueue.Entries > 0){
        auto Entry = *ToDoQueue.Entry->at(1);
        if(Entry.Key > ServerMilliseconds) break;
        ToDoQueue.deleteMin();
        TCreature *Creature = GetCreature(Entry.Data);
        if(Creature != NULL) Creature->Execute();
    }
}
```

This is a **global min-heap** keyed by `ServerMilliseconds` (logical time). Every creature
action (walk, attack, use, wait) is scheduled into this queue via `ToDoStart()`
(`cract.cc:955-968`):

```c
uint32 Delay = this->CalculateDelay();
if(Delay < 1) Delay = 1;
uint32 NextWakeup = ServerMilliseconds + Delay;
ToDoQueue.insert(NextWakeup, this->ID);
this->NextWakeup = NextWakeup;
```

`ServerMilliseconds` only advances in discrete `Beat`-sized steps inside `MoveCreatures`.
All scheduled actions are **quantized to beat boundaries** — a walk scheduled for `t+150ms`
on a 200 ms beat executes at the next beat where `ServerMilliseconds >= t+150`. The Rust
`ToDoQueue` (`creature_todo.rs`) reproduces this exactly; `now_ms()` returns `server_ms`
unconditionally (Phase 6 collapsed the wall-clock vs `server_ms` fork).

### 2.5 Walk Speed Quantization

```c
// cract.cc:1459-1463 (reference)
int Delay = (Waypoints * 1000) / this->GetSpeed();
int BeatCount = (Delay + Beat - 1) / Beat;        // ceil to Beat
this->EarliestWalkTime = ServerMilliseconds + BeatCount * Beat;
```

Walk delays are rounded up to the nearest `Beat` multiple. The Rust `walk.rs` quantizes by
**`profile.step_beat_ms`** (50 ms for both shipped eras — TVP `gameserver` authority for
772), not by `beat_ms` (the 200 ms main-loop timer). `StepSpeedModel` selects the raw
delay curve:

- `LinearGo` (772): `(gs * 1000) / eff_speed`, then ceil to `step_beat_ms`.
- `TfsLog` (1098): TFS `getStepDuration` log curve, then ceil to `step_beat_ms`.

See `tasks/lessons.md` §30 (B1) for why `step_beat_ms` (not `beat_ms`) is the walk
quantizer.

### 2.6 `SendAll` — Consolidated Output

```c
// sending.cc:17-33 (reference)
void SendAll(void){
    TConnection *Connection = FirstSendingConnection;
    FirstSendingConnection = NULL;
    while(Connection != NULL){
        if(Connection->WillingToSend){
            Connection->WillingToSend = false;
            if(Connection->Live() && Connection->NextToCommit > Connection->NextToSend){
                tgkill(GetGameProcessID(), Connection->GetThreadID(), SIGUSR2);
            }
        }
        Connection = Connection->NextSendingConnection;
    }
}
```

Output is written into per-connection ring buffers during game logic. `SendAll` signals
each I/O thread (`SIGUSR2`) that data is ready; the I/O thread encrypts and writes to TCP.
In the unified engine, `flush_pending_outgoing` at beat end is the `SendAll` equivalent.
The profile flush policy determines whether movement packets also flush immediately (1098)
or wait for beat end (772).

### 2.7 `ReceiveData` — Input Processing

```c
// receiving.cc:1796-1812 (reference)
void ReceiveData(void){
    TConnection *Connection = GetFirstConnection();
    while(Connection != NULL){
        if(Connection->Live() && Connection->WaitingForACK){
            ReceiveData(Connection);               // parse + execute command
            Connection->WaitingForACK = false;
            if(Connection->Live()){
                tgkill(..., Connection->GetThreadID(), SIGUSR1);  // ACK to I/O thread
            }
        }
        Connection = GetNextConnection();
    }
}
```

Player commands are parsed and **executed immediately** on the game thread (not queued).
The actions they trigger (walk, attack, use) are scheduled into `ToDoQueue` with
appropriate delays. The resulting output packets are not flushed until `SendAll()` (772) or
flushed immediately for movement (1098, via profile flush policy).

---

## 3. Per-Era Profile Knobs

The loop reads these from `MechanicsProfile` (+ `data/formulas/<v>.lua`); none are
hardcoded in the loop body.

| Knob | 772 (default) | 1098 | C++ source |
|------|---------------|------|------------|
| `beat_ms` | 200 | 50 | `config.cc:100` (`Beat = 200`); 1098 `EVENT_CREATURE_THINK_INTERVAL` |
| `step_beat_ms` (walk quantizer) | 50 | 50 | TVP `gameserver` (772); TFS `getStepDuration` (1098) |
| `step_speed` (`StepSpeedModel`) | `LinearGo` | `TfsLog` | `cract.cc:1459-1463` (772); `creature.cpp:1485-1547` (1098) |
| Think cadence | staggered ~1000 ms (`ProcessCreatures` counter) | 50 ms bucketed | `main.cc:185-188` (772); `game.cpp:3819-3850` (1098) |
| Condition/skill tick interval | ~1000 ms (`ProcessSkills` counter) | 50 ms | `main.cc:197-200` (772); `game.cpp:3819-3850` (1098) |
| Flush policy | beat-end only (`SendAll`) | immediate-on-movement + tick-end | `sending.cc:17-33` (772); inline per handler (1098) |
| `parity_rng_source` | `PerWorldGlibc` | `EnvGlobal` | Phase 6 K1 |
| `corpse_decay_offset_ms` | 30 000 | 600 | Phase 6 K2 |
| `underground_sees_surface` | true | false | Phase 6 K3 |
| `damage_text_format` | `AttackerAttribution` | `SimpleLoss` | Phase 6 K10 |
| `periodic_ping_packet` | codec-selected | codec-selected | Phase 6 X (`ProtocolCodec::periodic_ping_packet`) |

**Adding a new era:** set these knobs in `MechanicsProfile::for_version(N)` +
`data/formulas/N.lua` and add a `Codec` impl in `tfs-rust-net`. Zero core branches. If a
core `if version == …` is ever needed, the difference belongs in the profile or codec —
re-classify per `tasks/unified-beat-engine-phases.md` Phase 1.

---

## 4. C++ Reference Index

Both eras still cite their authoritative C++ sources. The unified engine is *shaped* after
the 772 beat loop (the more constrained architecture); 1098 observable behavior is matched
by tuning the profile knobs above.

| Concept | 1098 Reference | 772 Reference |
|---------|---------------|---------------|
| Main loop | `src/tasks.cpp:21-46` (Dispatcher) | `tibia-game-master/src/main.cc:456-492` (`LaunchGame`) |
| Timer/scheduler | `src/scheduler.cpp:10-36` | `tibia-game-master/src/main.cc:144-168` (`InitTime`) |
| Tick pipeline | `src/game.cpp:3819-3850` (`checkCreatures`) | `tibia-game-master/src/main.cc:312-449` (`AdvanceGame`) |
| Creature movement | `src/game.cpp:3773-3779` (`checkCreatureWalk`) | `tibia-game-master/src/crmain.cc:1106-1122` (`MoveCreatures`) |
| Action scheduling | `src/creature.cpp:318-321` (`addEventWalk`) | `tibia-game-master/src/cract.cc:955-968` (`ToDoStart`) |
| Action execution | `src/creature.cpp:236-308` (`onWalk`) | `tibia-game-master/src/cract.cc:728-843` (`Execute`) |
| Walk speed calc | `src/creature.cpp:1485-1547` (`getStepDuration`) | `tibia-game-master/src/cract.cc:1459-1463` (`NotifyGo`) |
| Output flush | Inline per handler | `tibia-game-master/src/sending.cc:17-33` (`SendAll`) |
| Input processing | `src/tasks.cpp:37-41` (Dispatcher drain) | `tibia-game-master/src/receiving.cc:1796-1812` (`ReceiveData`) |
| I/O → game thread | `g_dispatcher.addTask()` | `CallGameThread()` → `SIGUSR1` (`communication.cc:650-662`) |
| Beat config | N/A (Dispatcher is event-driven) | `tibia-game-master/src/config.cc:100` (`Beat = 200`) |
| Idle / AI tick | `src/monster.cpp:759` (`onIdleStimulus` — think-driven) | `tibia-game-master/src/crnonpl.cc:2386` (`IdleStimulus` on ToDo drain) — see [`IDLE_STIMULUS.md`](IDLE_STIMULUS.md) |
| ToDoQueue | N/A (per-creature timers) | `tibia-game-master/src/cr.hh:937` (`priority_queue<uint32, uint32>`) |

---

## 5. Implementation Boundary

### What stays shared (both eras, single engine)

- `run_game_loop` — the only loop entry point
- `GameWorld` struct and all entity storage
- `SlotMap` creature/item management
- Map, tiles, pathfinding
- Global `ToDoQueue` min-heap keyed by `server_ms`
- `advance_beat` staggered subsystem counters (thresholds/cadence profile-driven)
- Packet encoding (`tfs-rust-net` — codec selects wire format)
- DB persistence (`tfs-rust-db`)
- I/O thread architecture (Tokio tasks + mpsc channels)

### What diverges by era (all via `MechanicsProfile` / `ProtocolCodec`)

| Component | 772 profile value | 1098 profile value |
|-----------|-------------------|--------------------|
| `beat_ms` | 200 | 50 |
| `step_speed` | `LinearGo` | `TfsLog` |
| Think cadence | staggered ~1000 ms | 50 ms bucketed |
| Skill/condition interval | ~1000 ms | 50 ms |
| Flush policy | beat-end only | immediate-on-movement + tick-end |
| Wire bytes / opcodes | `Codec772` | `Codec1098` |
| RNG source / corpse decay / sight / damage text | see §3 | see §3 |

No loop-selection branch exists in `run_server.rs` or anywhere else in core. The loop is
selected once at startup: `run_game_loop(world, cmd_rx, out_registry)`.
