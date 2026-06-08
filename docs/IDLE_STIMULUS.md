# IdleStimulus — High-Level Implementation Plan

> **Status:** Phase A implemented (772 drain-triggered idle + Go). Phase B (Attack) deferred.
>
> **Related:** [`GAME_LOOP_ARCHITECTURE.md`](GAME_LOOP_ARCHITECTURE.md) §3 (772 loop),
> [`PROTOCOL_VERSIONING.md`](PROTOCOL_VERSIONING.md) §12.1 / §12.4,
> [`CODEBASE_AUDIT.md`](CODEBASE_AUDIT.md) §4 / §9.

---

## 1. What IdleStimulus Is

In 772, creature AI is **not** a periodic “think every N ms” loop. The real AI tick runs
when a creature’s **ToDo action list drains**:

```
ToDoQueue drain → Execute() runs queued actions (Go / Attack / Wait / Talk)
               → queue empty → IdleStimulus() (virtual)
               → enqueue next actions + CalculateDelay
               → ToDoStart() → global heap wakeup
```

C++ references:

| Concept | Source |
|---------|--------|
| Global heap drain | `crmain.cc:1106` `MoveCreatures` |
| Per-creature execute | `cract.cc:728` `TCreature::Execute` |
| Schedule next wakeup | `cract.cc:955` `ToDoStart`, `CalculateDelay` (`cract.cc:846`) |
| Monster idle AI | `crnonpl.cc:2386` `TMonster::IdleStimulus` |
| Player idle AI | `crmain.cc` `TPlayer::IdleStimulus` |

TFS 1.4.2 replaced this with scheduler-driven `onThink` + walk deadlines (`creature.cpp`,
`monster.cpp:759` `onIdleStimulus` — name only; timing model differs). Our **1098** path correctly
keeps that model. **772** should converge on drain-triggered idle, not transcribe C++ vectors.

---

## 2. Current Rust State

| Layer | 772 today | CipSoft target |
|-------|-----------|----------------|
| Global scheduler | `ToDoQueue` + `server_ms` + `advance_beat_772` | `ToDoQueue` min-heap |
| Walk execution | `process_creature_todo` → `on_walk` | `Execute` → `TDGo` |
| Walk delay math | CipSoft speed + beat quantizer (`walk.rs`) | `NotifyGo` / `Beat=200` |
| **AI trigger** | `check_creatures` → `monster_on_think` (~1000 ms buckets) | **Idle on ToDo drain** |
| Chase / repath | Think-driven + `monster_arm_event_walk` | Idle enqueues `ToDoGo` |
| Attack | `creature_on_attacking` from think bucket; `doAttacking` stub | Idle enqueues `ToDoAttack` |

Key files today:

- `todo_queue.rs` — global heap (P2)
- `walk.rs` — `drain_todo_queue`, `process_creature_todo`, `add_event_walk` (walk-only execute path)
- `creature_think.rs` — TFS bucketed think for **both** eras
- `monster_ai.rs` — targeting, flee, repath, `monster_arm_event_walk` (1098-oriented arm gate)

**Gap:** Walk is heap-driven; **decisions about what to do next** still come from TFS think. That
produces wrong chase cadence and redundant re-arming on 772 (see audit P7).

---

## 3. Target Architecture (772 Only)

IdleStimulus is **profile-gated**, not codec-gated. Enable when
`MechanicsProfile::beat_driven_loop` / `StepSpeedModel::LinearGo` — same flag as P2.

```
┌─────────────────────────────────────────────────────────────┐
│  advance_beat_772 → drain_todo_queue                        │
│       │                                                     │
│       ▼                                                     │
│  process_creature_todo(cid)                                 │
│       │                                                     │
│       ├─ pop next CreatureAction from per-creature queue    │
│       ├─ Execute arm (Go / Attack / Wait / …)               │
│       └─ if queue empty → idle_stimulus(cid)                │
│              └─ enqueue next actions → ToDoStart            │
└─────────────────────────────────────────────────────────────┘
```

### Rust shape (idiomatic, not C++ transcription)

```rust
enum CreatureAction {
    Go { /* direction or path intent */ },
    Attack { target: CreatureId },
    Wait { ticks: u32 },
    // Talk { ... } — NPC phase, later
}

struct CreatureTodo {
    queue: VecDeque<CreatureAction>,
    act_index: usize,
    locked: bool,           // C++ LockToDo while executing
}
```

- **Global** `ToDoQueue`: when to wake a creature (logical `server_ms`).
- **Per-creature** action queue: what to run when woken (enum deque, not `void*` task list).
- **`idle_stimulus`**: dispatches on `CreatureKind` — `monster_idle_stimulus`, `player_idle_stimulus`, etc.

1098 unchanged: `check_creatures`, Tokio walk timers, `next_walk_check`.

---

## 4. Phased Rollout

Combat is **not** a prerequisite for the architecture or for chase/walk parity.

### Phase A — Drain-triggered idle + Go (no combat)

**Goal:** Replace think-driven chase arming with idle-on-drain → enqueue Go.

| Work item | Notes |
|-----------|-------|
| Per-creature `CreatureTodo` queue | Insert/pop around existing walk path |
| `idle_stimulus(cid)` hook | Called when action queue drains after `Execute` |
| `monster_idle_stimulus` | Target search, flee, keep-distance, enqueue Go/repath |
| Retire 772 think chase arming | Narrow `monster_arm_event_walk` / think repath for `beat_driven_loop` |
| P7 fix | `monster_arm_event_walk` checks `next_wakeup`, not `next_walk_check` |
| Move follow repath into idle | Align with `follow_repath_without_path` (P3 done) |

**Does not require:** damage pipeline, spell cast, condition ticks.

**Verification:** Monster chase repaths on target move; step cadence matches beat quantizer; no
duplicate heap entries; idle fires only after queue drain, not on 50 ms / 1000 ms think tick.

### Phase B — Attack + Wait ToDo actions

**Goal:** Observable melee/ranged cadence from the queue, not parallel think attacking.

| Work item | Blocked on |
|-----------|------------|
| `CreatureAction::Attack` execute arm | Combat execution module (`combat/mod.rs` loop, not just `combat/math.rs`) |
| `CalculateDelay` for attacks | B4 math exists; needs wiring + `ToDoStart` |
| Retire 772 `creature_on_attacking` from think bucket | Phase B attack execute |
| `CreatureAction::Wait` | Optional; low priority |

### Phase C — NPC Talk + player idle (deferred)

- NPC `Talk` actions stay **Lua-first** (design §12.6); no `.ndb` engine.
- Player idle (auto-eat, idle regen triggers) when player ToDo model is needed.

---

## 5. Relationship to Other Work

| Item | Relationship |
|------|----------------|
| **P2** (772 loop MVP) | **Prerequisite** — heap + `server_ms` exist |
| **P3** (`follow_repath_without_path`) | **Done** — idle repath policy knob |
| **P4** (staggered subsystem counters) | **Orthogonal** — replaces 50 ms `on_tick` for regen/spawns/skills; not per-creature idle |
| **P7** (`next_wakeup` arm gate) | **Part of Phase A** — hygiene before or with idle hook |
| **B4 combat math** | **Done** — formulas only; Phase B needs execution loop |
| **B3 monster AI knobs** | **Done** — targeting/flee/distance feed into `monster_idle_stimulus` |

**Order recommendation:** P7 → Phase A (Go idle) in parallel with or before P4. Phase B after
combat executes. Do not block Phase A on combat.

---

## 6. What Stays on Think (772)

Even after IdleStimulus, some subsystems may remain on staggered global counters (P4), not idle:

| Subsystem | CipSoft cadence | Rust home |
|-----------|-----------------|-----------|
| Condition ticks | ~1000 ms stagger | P4 / `executeConditions` path |
| Regeneration | ~1000 ms | P4 |
| Spawn checks | ~1000 ms | P4 / `spawn_lifecycle.rs` |
| Lose-target roll | Per idle (CipSoft) | **Move to idle** in Phase A |
| Target search on opponent enter | Event-driven (`onCreatureMove`) | **Keep** — not idle |

Think bucket (`check_creatures`) on 772 should **shrink** to P4 subsystems + 1098 parity paths,
not drive monster chase.

---

## 7. Success Criteria

### Phase A

- [ ] Monster with empty ToDo queue after a walk step calls `idle_stimulus`, not `monster_on_think`.
- [ ] Chase repath enqueues Go from idle when target moves (772 `follow_repath_without_path`).
- [ ] `next_wakeup` set once per scheduled action; stale heap entries skipped (existing guard).
- [ ] 1098 regression: `check_creatures` + Tokio walk unchanged.
- [ ] Unit/integration tests on `minimal_world` with `beat_driven_loop = true`.

### Phase B

- [ ] Melee attack only when Attack action executes at heap time.
- [ ] Attack delay uses profile attack speed (772 flat 2000 ms from B4).
- [ ] No double-hit from think + ToDo paths.

---

## 8. Anti-Patterns

| Avoid | Why |
|-------|-----|
| IdleStimulus on every beat for all creatures | O(n) scan; CipSoft is per-creature on drain |
| Keeping `walk_wake_tx` **and** ToDo for 772 | Dual schedulers (audit §8) |
| Codec/version `if 772` in core | Use `beat_driven_loop` / profile |
| Copying C++ `ToDoList` vector layout | Use Rust enum queue; same outcomes |
| Waiting for full combat before Phase A | Go/chase is the main cadence gap |
| Running idle from 50 ms hybrid `on_tick` | Wrong trigger; use drain only |

---

## 9. File Touch Map (When Implementing)

| File | Phase A | Phase B |
|------|---------|---------|
| New: `creature_todo.rs` or `idle_stimulus.rs` | Queue + idle dispatch | Attack/Wait arms |
| `walk.rs` | Wire execute → pop Go; drain → idle | Attack delay scheduling |
| `monster_ai.rs` | `monster_idle_stimulus`; shrink think chase | Attack enqueue from idle |
| `creature_think.rs` | Skip monster chase on `beat_driven_loop` | Skip `creature_on_attacking` on 772 |
| `game_loop.rs` | No change expected | No change expected |
| `creature/base.rs` | `CreatureTodo` fields | — |

---

## 10. References

- CipSoft: `tibia-game-master/src/crnonpl.cc` (`IdleStimulus`), `cract.cc` (`Execute`, `ToDoStart`)
- TFS 1.4.2: `src/creature.cpp` (`executeToDoEntries`), `src/monster.cpp` (`onIdleStimulus`)
- Rust today: `todo_queue.rs`, `walk.rs`, `creature_think.rs`, `monster_ai.rs`
- Audit: [`CODEBASE_AUDIT.md`](CODEBASE_AUDIT.md) §4 row `creature_think.rs`, §9 P7
