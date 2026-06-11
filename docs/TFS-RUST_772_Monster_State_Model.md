# 772 Monster State Model — Design Decision

**Date:** 2026-06-11  
**Status:** Decision doc (pre-implementation)  
**Related:**
- [TFS-RUST_772_Monster_AI_Comprehensive_Gap_Audit.md](TFS-RUST_772_Monster_AI_Comprehensive_Gap_Audit.md) — Phase D (D1/D2), gaps I1/E5/H2/S3
- [PROTOCOL_VERSIONING.md](PROTOCOL_VERSIONING.md) — 772 mechanics vs 1098 TFS

---

## Executive summary

Rust monsters use a single boolean, `is_idle`, for both TFS 1098 idle semantics and 772 sleep/wake. The 772 reference uses a distinct `STATE` enum where **`SLEEPING` and `IDLE` are separate states** with different wake rules, spawn behavior, and todo preemption.

**Verdict:** the current `is_idle`-only approach is **not sufficient for 772 parity**. Phase D should add a **minimal 772 lifecycle field** (`Sleeping` vs `Awake`). A full `MonsterState` enum with combat states (`UNDERATTACK`, `PANIC`, `ATTACKING`) can wait until Phase E unless combat work starts immediately after D.

---

## Reference state machine

772 non-player creatures carry a `STATE` enum (`enums.hh`):

| Value | Name | Role |
|-------|------|------|
| 0 | `SLEEPING` | Spawn default; skips full `IdleStimulus` body until woken |
| 1 | `IDLE` | Awake; full idle drain (targeting, movement, combat tail) |
| 2 | `UNDERATTACK` | Hit while not already in combat posture |
| 3 | `TALKING` | NPC/dialogue (monsters rarely) |
| 4 | `LEAVING` | Logout/despawn in progress |
| 5 | `ATTACKING` | Melee posture; gates some `ToDoGo` paths |
| 6 | `PANIC` | Flee posture after damage |

Phase D scope uses only **`SLEEPING` ↔ `IDLE`**. Combat states are Phase E5.

### Spawn

```cpp
// crnonpl.cc:1516–1517 — TNonplayer constructor
TNonplayer::TNonplayer(void) : TCreature() {
    this->State = SLEEPING;
}
```

No target acquisition runs at construction. The monster stays asleep even if players are already in scan range.

### Idle early exit (re-sleep)

During `IdleStimulus` target scan (`crnonpl.cc:2420–2507`), when `Target == 0`:

1. `TFindCreatures(12, 12)` scans players and monsters.
2. Wild monsters (`MONSTER && !IsPlayerControlled()`) are skipped for targeting but **still clear `ShouldSleep`** if visible on the same floor (`CanSeeFloor`).
3. If after the scan `ShouldSleep && Target == 0` and state is not `UNDERATTACK` / `PANIC`:
   - Wild (no master): `State = SLEEPING`; return.
   - Summon: `ToDoWait(1000)` + `ToDoStart()`; return.

So **`SLEEPING` is re-entered** when no qualifying creatures remain — not merely "no current target."

### Wake on move (`CreatureMoveStimulus`)

```cpp
// crnonpl.cc:2866–2894
void TMonster::CreatureMoveStimulus(uint32 CreatureID, int Type) {
    if (CreatureID == this->ID) {
        if (this->State == SLEEPING && Type != OBJECT_DELETED) {
            this->State = IDLE;
            this->ToDoYield();
        }
        return;
    }

    if (this->State == SLEEPING && Type != OBJECT_DELETED) {
        TCreature *Creature = GetCreature(CreatureID);
        if (Creature->Type == NPC) return;
        if (Creature->Type == MONSTER && !((TMonster*)Creature)->IsPlayerControlled())
            return;

        this->State = IDLE;
        this->ToDoYield();
    }

    TCreature::CreatureMoveStimulus(CreatureID, Type);
}
```

Key behaviors:

| Trigger | Reference | Rust today |
|---------|-----------|------------|
| Self-move while sleeping | `SLEEPING → IDLE` + `ToDoYield` | Self-move only updates opponent list / idle status |
| Player moves (already visible) | Wake + yield | **No wake** — only viewport enter adds opponents |
| Player-controlled summon moves | Wake + yield | Same gap |
| Wild monster moves | Ignored | N/A (no separate wake path) |
| NPC moves | Ignored | N/A |
| Wake side effect | `ToDoYield()` only — **no immediate chase** | Viewport enter may call `monster_try_acquire_chase_target` |

### `ToDoYield`

```cpp
// cract.cc:1001–1005
void TCreature::ToDoYield(void) {
    if (!this->LockToDo) {
        this->ToDoWait(0);
        this->ToDoStart();
    }
}
```

Enqueue `Wait(0)` and reschedule execution on the global todo heap. This **preempts** in-flight actions. Rust's `request_idle_stimulus` is different: it returns early when the todo queue is non-empty or a walk timer is active.

---

## Rust today

### `Monster.is_idle`

Defined in `crates/tfs-rust-core/src/creature/monster.rs`:

```rust
pub is_idle: bool,  // default: true at spawn
```

Used for:

| Concern | Gate |
|---------|------|
| Skip `monster_idle_stimulus` body | `if is_idle { return; }` (`idle_stimulus.rs`) |
| Exclude from think sweep | `remove_creature_think_check` when idle |
| TFS random-walk suppression | `monster_next_walk_step` / `getNextStep` |
| Opponent-list hygiene | `monster_update_idle_status` → `is_idle = opponent_ids.is_empty()` |

### What `is_idle` conflates

| Reference concept | Rust `is_idle` |
|-------------------|----------------|
| `SLEEPING` at spawn | `is_idle = true` |
| `IDLE` after wake, no target yet | `is_idle = false` (once opponents exist) |
| Re-sleep when area empty | `is_idle = true` when `opponent_ids` empty |
| Post-combat idle (772 `IDLE`) | Same boolean |

The conflation is mostly harmless on **1098** (TFS uses one idle flag). On **772**, it breaks three observable behaviors:

1. **Spawn chase** — `spawn_lifecycle.rs` calls `monster_on_creature_appear_self` after place, which scans spectators, populates `opponent_ids`, sets `is_idle = false`, and may acquire chase **before any move stimulus**.
2. **Move wake scope** — wake only happens on viewport **enter** (`can_see_new && !can_see_old`), not on every qualifying move while asleep.
3. **Todo preemption** — no `ToDoYield` equivalent on wake.

### Existing opponent logic (reusable)

`monster_is_opponent` in `monster_targets.rs` already mirrors player / player-summon filtering for list membership:

- Players (respecting ghost / ignored-by-monsters flags)
- Summons whose master is a player
- **Not** wild monsters

This can back a new `monster_is_move_wake_source` helper for D1 without duplicating policy.

---

## Behavioral matrix

| Event | Reference (772) | Rust `is_idle` today | Phase D target |
|-------|-----------------|----------------------|----------------|
| Monster spawned near player | `SLEEPING`, no target | May chase immediately | `Sleeping`, no chase until wake |
| Player walks into viewport | Wake via move stimulus + yield | `is_idle=false`, may chase on enter | `Sleeping → Awake` + `todo_yield`; chase after idle drain |
| Player moves while already visible | Wake + yield | No state change if already opponent | `Sleeping → Awake` + `todo_yield` |
| Wild rat moves near sleeping rat | No wake | No wake | No wake |
| All opponents leave | Re-sleep scan → `SLEEPING` | `is_idle=true`, clears lists | `Awake → Sleeping` when scan says `ShouldSleep` |
| Damage while sleeping | `PANIC`/`UNDERATTACK` + yield (Phase E) | No handler | Deferred to Phase E |
| 1098 monster idle | N/A | `is_idle` unchanged | No change on 1098 path |

---

## Design options

### Option A — Keep `is_idle` only (not recommended)

Patch wake by setting `is_idle = false` on more move paths.

**Problems:**

- Cannot represent "awake but no target yet" vs "sleeping" differently for re-sleep.
- Spawn chase requires ad-hoc flags ("don't acquire on first appear").
- Phase E combat states have nowhere to live.
- 1098 and 772 semantics stay entangled.

### Option B — Minimal lifecycle enum (recommended for Phase D)

Add a 772-only lifecycle field on `Monster`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MonsterLifecycleState {
    #[default]
    Sleeping,  // reference SLEEPING
    Awake,     // reference IDLE (non-combat)
}
```

| Layer | 1098 | 772 (`beat_driven_loop`) |
|-------|------|--------------------------|
| Sleep gate | `is_idle` | `lifecycle == Sleeping` |
| Think sweep | `!is_idle` | `lifecycle == Awake` |
| Idle stimulus | skip if `is_idle` | skip if `Sleeping` |
| Spawn | current behavior | `Sleeping`, no immediate chase acquire |
| Wake | N/A (TFS enter path) | move stimulus → `Awake` + `todo_yield` |

Keep `is_idle` for 1098. On 772, either ignore `is_idle` for gating or derive it from lifecycle for logging only — **one source of truth per era**.

**Pros:** Small diff, closes D1/D2, clear Phase E extension point.  
**Cons:** Two fields on `Monster` until Phase E merges or namespaces combat into a richer enum.

### Option C — Full `MonsterState` enum now

Mirror all reference `STATE` values immediately.

**Pros:** One enum for all phases; Phase E gating is trivial.  
**Cons:** Larger diff now; most variants unused until combat; more call-site churn before D1/D2 land.

---

## Recommendation

**Ship Option B for Phase D.** Add `MonsterLifecycleState` gated by `beat_driven_loop`. Defer full `MonsterState` until Phase E unless combat work follows D immediately.

### New helpers (implementation reference, not in scope for this doc)

| Helper | Role |
|--------|------|
| `creature_todo_yield(cid)` | `ToDoWait(0)` + `schedule_immediate_todo_wakeup`; respect `todo.locked` |
| `monster_is_move_wake_source(cid)` | Player or player-controlled summon; not NPC, not wild monster |
| `monster_wake_from_sleep(cid)` | `Sleeping → Awake`, add think check, `creature_todo_yield` |
| `monster_creature_move_stimulus(...)` | 772 hook in `monster_on_creature_move` before TFS list updates |

### Spawn change (D2)

On 772, after `spawn_monster` places the creature:

- Set `lifecycle = Sleeping`.
- Run `monster_update_target_list` only (optional: populate list for later).
- **Do not** call `monster_try_acquire_chase_target` while `Sleeping`.

### Re-sleep (D2 tail)

In `monster_idle_stimulus` target scan (Phase C will replace search; until then):

- When no target and no visible qualifying creatures on floor → `Sleeping`.
- Mirror `ShouldSleep` logic from `crnonpl.cc:2454–2456` (visible player/player-monster on same floor clears sleep intent).

---

## Call-site migration (Option B)

| File | Change |
|------|--------|
| `creature/monster.rs` | Add `MonsterLifecycleState`, default `Sleeping` |
| `idle_stimulus.rs` | Gate on `Sleeping` when `beat_driven_loop` |
| `monster_events.rs` | `monster_creature_move_stimulus` before TFS move handling |
| `monster_targets.rs` | `monster_wake_from_sleep`, gate chase while `Sleeping` |
| `spawn_lifecycle.rs` | 772 spawn: no immediate chase acquire |
| `creature_todo.rs` | `creature_todo_yield` |
| `creature_think.rs` | Think bucket: `Awake` on 772 instead of `!is_idle` |
| `monster_ai.rs` | Tests / 1098 paths: keep `is_idle` |

Combat-gated sites (Phase E — document only):

| Reference state | Future Rust |
|-----------------|-------------|
| `UNDERATTACK` | `MonsterState::UnderAttack` or lifecycle + combat flags |
| `PANIC` | `MonsterState::Panic` — skip melee `ToDoGo` |
| `ATTACKING` | `MonsterState::Attacking` — enqueue `ToDoAttack` tail |

---

## `is_idle` vs lifecycle — coexistence rules

To avoid dual sources of truth on 772:

1. **`beat_driven_loop == false` (1098):** `is_idle` remains authoritative. Ignore `lifecycle` or keep it `Awake`.
2. **`beat_driven_loop == true` (772):** `lifecycle` is authoritative for sleep/wake and idle_stimulus gating.
3. **Do not** set `is_idle = false` on 772 wake; set `lifecycle = Awake`.
4. **Logging:** expose both in debug traces during migration; converge later.

---

## Verification plan (when implementing Phase D)

| Test | Asserts |
|------|---------|
| `test_772_spawn_stays_sleeping_with_player_in_view` | Spawn adjacent to player → `Sleeping`, no `follow_target`, no todo `Go` |
| `test_772_player_move_wakes_sleeping_monster` | Player visible, moves 1 tile → `Awake`, yield scheduled |
| `test_772_wild_monster_move_does_not_wake` | Wild monster move → stays `Sleeping` |
| `test_772_self_move_wakes_sleeping_monster` | Own move while `Sleeping` → `Awake` |
| `test_creature_todo_yield_preempts_queue` | In-flight todo + yield → `Wait(0)` reschedules |

```bash
cargo test -p tfs-rust-core --lib test_772_spawn_stays_sleeping -- --nocapture
cargo test -p tfs-rust-core --lib test_772_player_move_wakes -- --nocapture
cargo test -p tfs-rust-core --lib test_creature_todo_yield -- --nocapture
```

---

## Open questions (defer unless blocking)

| Question | Default for Phase D |
|----------|---------------------|
| Parse `ShouldSleep` per race from `.mon` / `RaceData`? | Treat all wild monsters as sleep-capable; summons use master wait path later |
| `monster_update_target_list` while `Sleeping` — populate `opponent_ids`? | Yes for list hygiene; do not wake or chase |
| 1098 spawn near player — change behavior? | No — 1098 keeps `monster_on_creature_appear_self` |

---

## Changelog

| Date | Change |
|------|--------|
| 2026-06-11 | Initial decision doc — `is_idle` insufficient for 772; recommend `MonsterLifecycleState` for Phase D |
