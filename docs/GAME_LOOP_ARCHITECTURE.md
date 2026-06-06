# Game Loop Architecture

This document defines the threading model and game loop design for both supported eras.
The 1098 loop matches TFS 1.4.2 `Dispatcher`/`Scheduler`. The 772 loop matches the CipSoft
decompiled server (`tibia-game-master/src/main.cc`). **One binary, two loop modes** — selected
by `clientVersion` in `config.lua`.

> **Implementation status (2026-06-06)**
>
> | Section | Status |
> |---------|--------|
> | **§2 — Era 1098 loop** | **Implemented.** `run_game_loop_1098` for `clientVersion = 1098`. |
> | **§3 — Era 772 loop** | **Implemented (P2 MVP).** `run_game_loop_772`: `ToDoQueue`, `server_ms`, `beat_ms` timer, beat-end flush. Hybrid 50 ms `on_tick` for subsystems; staggered counters still deferred. |
>
> See [`CODEBASE_AUDIT.md`](CODEBASE_AUDIT.md) for the full gap analysis.

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

## 2. Era 1098 — TFS Dispatcher Loop (Current Implementation)

### 2.1 C++ Reference Model

TFS 1.4.2 uses a **Dispatcher + Scheduler** pair:

| Component | C++ File | Behaviour |
|-----------|----------|-----------|
| `Dispatcher` | `src/tasks.cpp:21-46` | Single-threaded FIFO task queue. Blocks on `condition_variable` when empty, drains all pending tasks in batch, executes each inline. |
| `Scheduler` | `src/scheduler.cpp:10-36` | Per-event `boost::asio::steady_timer`. When timer fires, posts the task's callback into the `Dispatcher` queue via `g_dispatcher.addTask()`. |
| `Game::checkCreatures` | `src/game.cpp:3819-3850` | Recurring scheduler task at `EVENT_CREATURE_THINK_INTERVAL` (1000ms). Processes `onThink` / `onAttacking` / `executeConditions` for a bucket of creatures. |
| `Game::checkCreatureWalk` | `src/game.cpp:3773-3779` | Per-creature scheduler task. Timer fires at `getEventStepTicks()` delay → `Creature::onWalk()` + `Game::cleanup()`. |
| Player commands | `src/protocolgame.cpp` | I/O thread parses packet → `g_dispatcher.addTask(playerMove/...)`. Executes on Dispatcher thread **immediately** in FIFO order (no tick alignment). |

**Key property:** Commands execute the instant they reach the Dispatcher — there is no batching
to tick boundaries. Walk timers fire with sub-millisecond precision via `steady_timer`. Output
packets are sent inline from each handler (no consolidated flush).

### 2.2 Rust Implementation

File: `crates/tfs-rust-core/src/game_loop.rs`

```rust
loop {
    tokio::select! {
        biased;

        // Branch 1: Network commands + DB callbacks (= Dispatcher FIFO)
        cmd = cmd_rx.recv() => { /* process immediately */ }

        // Branch 2: Walk wake timers (= Scheduler steady_timer → Dispatcher)
        w = walk_wake_rx.recv() => {
            world.process_walk_due_from_wake(cid);
            flush_pending_outgoing(&mut world, &out_registry);
        }

        // Branch 3: Periodic world tick (= checkCreatures recurring event)
        _ = tick_timer.tick() => {   // 50ms interval
            world.on_tick(Instant::now());
            flush_pending_outgoing(&mut world, &out_registry);
        }
    }
}
```

**Mapping to C++:**

| Rust | C++ Equivalent |
|------|----------------|
| `cmd_rx` (unbounded mpsc) | `g_dispatcher` task queue |
| `walk_wake_rx` + `tokio::time::sleep_until` | `g_scheduler.addEvent(checkCreatureWalk)` |
| `tick_timer` (50ms interval) | `g_scheduler.addEvent(checkCreatures, EVENT_CREATURE_THINK_INTERVAL)` |
| `biased;` cmd-first ordering | Dispatcher FIFO: `playerMove` arriving first always runs before `checkCreatureWalk` |
| `game_packet_needs_immediate_flush()` | TFS dispatches reply packets inline from each handler |
| `pending` VecDeque (Turn→Move coalescing) | No C++ equivalent — Rust-specific optimisation for OTC turn+move input coalescing |

**Flush behaviour:** Movement and time-sensitive packets flush immediately
(`game_packet_needs_immediate_flush`). All other output flushes at tick end. This matches TFS
where each handler writes to the socket inline.

### 2.3 Subsystem Timing (1098)

| Subsystem | Interval | Implementation |
|-----------|----------|----------------|
| World tick (`on_tick`) | 50ms | `tokio::time::interval` |
| Creature think/attack/conditions | Every tick (bucketed across 10 groups) | `GameWorld::check_creatures()` |
| Walk scheduling | Per-creature `tokio::time::sleep_until` | `walk_wake_tx` → `walk_wake_rx` |
| Decay | Every tick | `DecayManager::tick()` |
| Spawn respawn polling | Every tick | `GameWorld::poll_spawn_respawns()` |
| Lua GC step | Every 5 ticks (250ms) | `events.lua_gc_step()` |
| Player ping | Every tick | `GameWorld::tick_player_pings()` |

---

## 3. Era 772 — CipSoft Beat-Driven Loop (Target Architecture)

### 3.1 C++ Reference Model

Source: `tibia-game-master/src/main.cc:456-492`, `cract.cc`, `config.cc`.

The CipSoft 7.72 server uses a fundamentally different architecture: a **signal-driven beat
loop** with a global **priority-queue scheduler** (`ToDoQueue`) and a **consolidated output
flush** (`SendAll`).

#### 3.1.1 The Main Loop (`LaunchGame`)

```c
// main.cc:477-492
while(GameRunning()){
    // 1. SLEEP until woken by a signal
    while(SigUsr1Counter == 0 && SigAlarmCounter == 0){
        SigWaitAny();           // sigsuspend — blocks until any signal
    }

    // 2. PROCESS INPUT (SIGUSR1 from I/O threads)
    if(SigUsr1Counter > 0){
        SigUsr1Counter = 0;
        ReceiveData();          // drain ALL pending player packets
    }

    // 3. ADVANCE GAME (SIGALRM from beat timer)
    int NumBeats = SigAlarmCounter;
    if(NumBeats > 0){
        SigAlarmCounter = 0;
        AdvanceGame(NumBeats * Beat);   // may accumulate multiple beats
    }
}
```

The game thread alternates between two wake sources:
- **`SIGUSR1`** — fired by I/O threads (`CallGameThread` in `communication.cc:650-662`) when
  a player packet arrives. The game thread drains all pending input via `ReceiveData()`.
- **`SIGALRM`** — fired by a POSIX `timer_t` at `Beat` intervals (`InitTime` in
  `main.cc:144-168`). This advances the simulation.

**Input can arrive between beats.** The game thread wakes on `SIGUSR1` and processes player
commands immediately. However, no output is flushed until the next `SendAll()` at the end of
`AdvanceGame`.

#### 3.1.2 The Beat Timer

```c
// config.cc:100
Beat = 200;    // default: 200ms (configurable in game config file)
```

The beat is a **200ms** POSIX monotonic timer (`CLOCK_MONOTONIC` + `SIGEV_THREAD_ID`), not a
50ms interval. This is the fundamental timing quantum of the CipSoft server.

#### 3.1.3 `AdvanceGame` — The Tick Pipeline

```c
// main.cc:312-449
static void AdvanceGame(int Delay){
    // Accumulate time into staggered counters
    CreatureTimeCounter += Delay;
    CronTimeCounter     += Delay;
    SkillTimeCounter    += Delay;
    OtherTimeCounter    += Delay;

    // 1. Creature think/regen (every ~1000ms)
    if(CreatureTimeCounter >= 1750){
        CreatureTimeCounter -= 1000;
        ProcessCreatures();
    }

    // 2. Cron system (every ~1000ms)
    if(CronTimeCounter >= 1500){
        CronTimeCounter -= 1000;
        ProcessCronSystem();
    }

    // 3. Skill events (every ~1000ms)
    if(SkillTimeCounter >= 1250){
        SkillTimeCounter -= 1000;
        ProcessSkills();
    }

    // 4. Connection management, spawns, ambient, etc. (every ~1000ms)
    if(OtherTimeCounter >= 1000){
        OtherTimeCounter -= 1000;
        RoundNr += 1;
        ProcessConnections();
        ProcessMonsterhomes();
        ProcessMonsterRaids();
        // ... light, network load checks, reboot schedule ...
    }

    // 5. Creature movement — drain priority queue
    if(Delay < 1000){
        MoveCreatures(Delay);
    }

    // 6. CONSOLIDATED OUTPUT FLUSH — once per beat
    SendAll();
}
```

**Key properties:**

- **Staggered subsystems:** Each subsystem has an independent counter with a different initial
  threshold (1750, 1500, 1250, 1000ms). They all reset by 1000ms. This staggers their first
  execution across different beats to spread CPU load.
- **`MoveCreatures` skipped during lag:** If `Delay >= 1000` (5+ missed beats), creature
  movement is suppressed entirely.
- **Single `SendAll()` at end:** ALL output for ALL connections is flushed exactly once per beat.
  No intermediate flushes.

#### 3.1.4 `MoveCreatures` — The ToDoQueue Priority Heap

```c
// crmain.cc:1106-1122
void MoveCreatures(int Delay){
    ServerMilliseconds += Delay;
    while(ToDoQueue.Entries > 0){
        auto Entry = *ToDoQueue.Entry->at(1);
        uint32 ExecutionTime = Entry.Key;
        uint32 CreatureID    = Entry.Data;
        if(ExecutionTime > ServerMilliseconds){
            break;                     // nothing due yet
        }

        ToDoQueue.deleteMin();
        TCreature *Creature = GetCreature(CreatureID);
        if(Creature != NULL){
            Creature->Execute();       // run the creature's ToDo list
        }
    }
}
```

This is a **global min-heap** keyed by `ServerMilliseconds` (logical time). Every creature
action (walk, attack, use, wait) is scheduled into this queue via `ToDoStart()`:

```c
// cract.cc:955-968
void TCreature::ToDoStart(void){
    if(this->NrToDo != 0){
        this->LockToDo = true;
        this->ActToDo = 0;

        uint32 Delay = this->CalculateDelay();
        if(Delay < 1) Delay = 1;

        uint32 NextWakeup = ServerMilliseconds + Delay;
        ToDoQueue.insert(NextWakeup, this->ID);
        this->NextWakeup = NextWakeup;
    }
}
```

`ServerMilliseconds` only advances in discrete `Beat`-sized steps inside `MoveCreatures`.
This means all scheduled actions are **quantized to beat boundaries** — a walk scheduled for
`t+150ms` on a 200ms beat will execute at the next beat where `ServerMilliseconds >= t+150`.

#### 3.1.5 Walk Speed Quantization

```c
// cract.cc:1459-1463
int Delay = (Waypoints * 1000) / this->GetSpeed();
int BeatCount = (Delay + Beat - 1) / Beat;        // ceil to Beat
this->EarliestWalkTime = ServerMilliseconds + BeatCount * Beat;
```

Walk delays are **rounded up to the nearest `Beat` multiple**. With `Beat=200`:
- A player with speed 220 on ground speed 150: `Delay = 150000/220 = 681ms` → `ceil(681/200)
  = 4` → `EarliestWalkTime = ServerMilliseconds + 800ms` (4 beats).

#### 3.1.6 `SendAll` — Consolidated Output

```c
// sending.cc:17-33
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

Output is written into per-connection ring buffers during game logic. `SendAll` signals each
I/O thread (`SIGUSR2`) that data is ready. The I/O thread then encrypts and writes to TCP. This
happens **exactly once per beat** — no intermediate flushes.

#### 3.1.7 `ReceiveData` — Input Processing

```c
// receiving.cc:1796-1812
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

Player commands are parsed and **executed immediately** on the game thread (not queued). The
actions they trigger (walk, attack, use) are scheduled into `ToDoQueue` with appropriate delays.
But the resulting output packets are not flushed until `SendAll()`.

### 3.2 Comparison Summary

| Aspect | 1098 (TFS) | 772 (CipSoft) |
|--------|------------|---------------|
| **Beat / tick interval** | 50ms | **200ms** (configurable) |
| **Input processing** | Immediate via `select!` | Immediate on `SIGUSR1` wake |
| **Action scheduling** | Per-creature `tokio::time::sleep_until` | Global `ToDoQueue` min-heap, keyed by `ServerMilliseconds` |
| **`ServerMilliseconds` advance** | Wall clock (`Instant::now()`) | Logical time, advanced in `Beat`-sized steps |
| **Walk timing** | Sub-ms precision via Tokio timers | Quantized to `Beat` multiples: `ceil(delay / Beat) * Beat` |
| **Output flush** | Immediate for movement; tick-end for rest | **Once per beat** — `SendAll()` at end of `AdvanceGame` |
| **Creature think** | Every 50ms tick (bucketed) | Every ~1000ms (`ProcessCreatures` staggered counter) |
| **Skill/condition ticks** | Every 50ms tick | Every ~1000ms (`ProcessSkills` staggered counter) |
| **Lag protection** | Tick overrun warning (45ms/50ms) | `MoveCreatures` suppressed if `Delay >= 1000ms` |

### 3.3 Rust Implementation Plan (772 Mode)

When `clientVersion = 772`, the game loop should switch to a beat-driven model that replicates
the CipSoft architecture. The key changes:

#### 3.3.1 Beat-Driven Main Loop

```rust
// Pseudocode for 772 game loop
let beat = Duration::from_millis(world.mechanics.profile.step_beat_ms as u64); // 200ms
let mut server_ms: u64 = 0;

loop {
    // Sleep until next beat or input arrives
    tokio::select! {
        biased;

        // Input: drain immediately (CipSoft SIGUSR1 equivalent)
        cmd = cmd_rx.recv() => {
            process_command(&mut world, cmd);
            // Do NOT flush output — wait for beat end
        }

        // Beat timer fires (CipSoft SIGALRM equivalent)
        _ = beat_timer.tick() => {
            server_ms += beat.as_millis() as u64;
            advance_game(&mut world, server_ms, beat);
            flush_pending_outgoing(&mut world, &out_registry);  // SendAll
        }
    }
}
```

#### 3.3.2 ToDoQueue — Global Priority Heap

Replace per-creature `walk_wake_tx` / `tokio::time::sleep_until` with a `BinaryHeap` keyed by
`server_ms`:

```rust
struct ToDoEntry {
    execution_time: u64,   // ServerMilliseconds when this fires
    creature_id: CreatureId,
}

struct ToDoQueue {
    heap: BinaryHeap<Reverse<ToDoEntry>>,  // min-heap
}
```

`MoveCreatures` equivalent:

```rust
fn drain_todo_queue(world: &mut GameWorld, server_ms: u64) {
    while let Some(&Reverse(entry)) = world.todo_queue.heap.peek() {
        if entry.execution_time > server_ms {
            break;
        }
        world.todo_queue.heap.pop();
        if let Some(creature) = world.creatures.get(entry.creature_id) {
            world.execute_creature_todo(entry.creature_id);
        }
    }
}
```

#### 3.3.3 Staggered Subsystem Counters

```rust
struct SubsystemCounters {
    creature_time: u64,  // threshold 1750, reset 1000
    cron_time: u64,      // threshold 1500, reset 1000
    skill_time: u64,     // threshold 1250, reset 1000
    other_time: u64,     // threshold 1000, reset 1000
}
```

#### 3.3.4 Consolidated Flush

In 772 mode, `game_packet_needs_immediate_flush()` always returns `false`. All output is
buffered into `pending_outgoing` during the entire beat and flushed once at beat end via
`SendAll` (the existing `flush_pending_outgoing`).

#### 3.3.5 Walk Speed Quantization

The walk speed formula already implements beat-aligned quantization in `walk.rs`:

```rust
// walk.rs — CipSoft step speed model (already implemented)
StepSpeedModel::CipSoft => {
    let delay = (gs as i64 * 1000) / i64::from(eff.max(1));
    ((delay + beat - 1) / beat) * beat   // ceil to Beat
}
```

In 772 mode, `EarliestWalkTime` must use `server_ms` (logical time) rather than
`Instant::now()` (wall time), and `ToDoQueue` must schedule the walk at that logical time.

---

## 4. Implementation Boundary

### What stays shared (both eras)

- `GameWorld` struct and all entity storage
- `SlotMap` creature/item management
- Map, tiles, pathfinding
- Packet encoding (`tfs-rust-net` — codec selects wire format)
- DB persistence (`tfs-rust-db`)
- I/O thread architecture (Tokio tasks + mpsc channels)

### What diverges by era

| Component | 1098 | 772 |
|-----------|------|-----|
| `run_game_loop()` | `tokio::select!` reactive loop | Beat-driven loop with `ToDoQueue` |
| Walk scheduling | `walk_wake_tx` + `tokio::time::sleep_until` | `ToDoQueue.insert(server_ms + delay, cid)` |
| Time source for walks | `Instant::now()` (wall clock) | `server_ms` (logical, Beat-stepped) |
| Output flush | Immediate for movement + tick-end | Beat-end only (`SendAll`) |
| Creature think interval | 50ms bucketed | ~1000ms staggered counters |
| Skill/condition interval | 50ms | ~1000ms |
| `game_packet_needs_immediate_flush` | Movement/turn/ping → `true` | Always `false` |
| `walk_wake_tx` | `Some(tx)` | `None` (unused) |

### Configuration

The loop mode is determined at startup from `MechanicsProfile`:

```rust
match world.mechanics.profile.step_speed {
    StepSpeedModel::TfsLog => run_game_loop_1098(world, cmd_rx, walk_wake_rx, out_registry),
    StepSpeedModel::CipSoft => run_game_loop_772(world, cmd_rx, out_registry),
}
```

---

## 5. C++ Reference Index

| Concept | 1098 Reference | 772 Reference |
|---------|---------------|---------------|
| Main loop | `src/tasks.cpp:21-46` (Dispatcher) | `tibia-game-master/src/main.cc:456-492` (LaunchGame) |
| Timer/scheduler | `src/scheduler.cpp:10-36` | `tibia-game-master/src/main.cc:144-168` (InitTime) |
| Tick pipeline | `src/game.cpp:3819-3850` (checkCreatures) | `tibia-game-master/src/main.cc:312-449` (AdvanceGame) |
| Creature movement | `src/game.cpp:3773-3779` (checkCreatureWalk) | `tibia-game-master/src/cract.cc:1106-1122` (MoveCreatures) |
| Action scheduling | `src/creature.cpp:318-321` (addEventWalk) | `tibia-game-master/src/cract.cc:955-968` (ToDoStart) |
| Action execution | `src/creature.cpp:236-308` (onWalk) | `tibia-game-master/src/cract.cc:728-843` (Execute) |
| Walk speed calc | `src/creature.cpp:1485-1547` (getStepDuration) | `tibia-game-master/src/cract.cc:1459-1463` (NotifyGo) |
| Output flush | Inline per handler | `tibia-game-master/src/sending.cc:17-33` (SendAll) |
| Input processing | `src/tasks.cpp:37-41` (Dispatcher drain) | `tibia-game-master/src/receiving.cc:1796-1812` (ReceiveData) |
| I/O → game thread | `g_dispatcher.addTask()` | `CallGameThread()` → `SIGUSR1` (`communication.cc:650-662`) |
| Beat config | N/A (Dispatcher is event-driven) | `tibia-game-master/src/config.cc:100` (`Beat = 200`) |
| ToDoQueue | N/A (per-creature timers) | `tibia-game-master/src/cr.hh:937` (`priority_queue<uint32, uint32>`) |
