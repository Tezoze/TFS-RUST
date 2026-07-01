# TFS-RUST 772 — Monster Collision / Bump-Pause Audit

**Date:** 2026-06-28
**Scope:** `crates/tfs-rust-core` monster collision, kick/push, and blocked-step recovery paths (772 / `beat_driven_loop == true` only)
**Symptom:** "When multiple monsters chase you on 772, they pause when bumping into each other."
**Reference:** `reference/cipsoft-772/tibia-game-master/src/` (decompile outcomes only — Rust is idiomatic, not transcribed)
**Related:**
- [`TFS-RUST_772_Monster_AI_Transition_Audit.md`](TFS-RUST_772_Monster_AI_Transition_Audit.md) — RC1/RC2 idle-transition stalls (different symptom, different seam; do not confuse)
- [`IDLE_STIMULUS.md`](IDLE_STIMULUS.md)
- [`GAME_LOOP_ARCHITECTURE.md`](GAME_LOOP_ARCHITECTURE.md)
- `tasks/lessons.md` entries #31, #77, #80

---

## 1. Executive Summary

The "monsters pause when bumping into each other" symptom on 772 is caused by **three divergences** between the Rust port and the 772 decompile, all in the **collision / blocked-step recovery seam** — distinct from the RC1/RC2 idle-transition stalls already fixed in `TFS-RUST_772_Monster_AI_Transition_Audit.md`.

The kick/push/kill *structure* was audited in Pass 8 (lesson #77) and is faithful, but three behavioral seams still don't match C++ outcomes:

| # | Severity | Finding | Subsystem | Symptom contribution |
|---|----------|---------|-----------|----------------------|
| **F1** | **Resolved (superseded)** | ~~Hard-block `Err` path doesn't re-run `IdleStimulus` next beat when an `Attack` is queued~~ — superseded by walk-engine unification Phase 1.3 (commit `c7b4df4`, 2026-07-01); see §4.1 and [`TFS-RUST_772_Monster_Audit_Verification.md`](TFS-RUST_772_Monster_Audit_Verification.md) §2.1 | blocked-step recovery | ~~Monster pauses 200 ms – 2000 ms+~~ No longer occurs — full `ToDoClear` + yield landed |
| **F2** | **High** | `KickCreature` dest validation uses `MovePossible(Execute=false)` (planning); C++ uses `Execute=true` (recursive chain-kick) | kick/push | No chain-push in dense convoys → kills / EXHAUSTED-pauses / creature stacking where C++ keeps everyone moving |
| **F3** | **Medium** | `monster_exhausted_wait_772` clears the target unconditionally; C++ `Execute` catch preserves it (only the player-tile case clears) | EXHAUSTED recovery | Blocked monster drops the player and re-acquires after 1 s instead of re-engaging the same target |

**Bottom line:** F1 was the primary single-monster stall cause but is now resolved. F2 is the remaining primary cause of the "bump pause" in dense groups — no chain-push so dense convoys stack/kill/stall where C++ flows. F3 amplifies the "they pause and seem to give up" feel by dropping aggro on top of the pause. F2+F3 compound with the RC1 fix (no 1 Hz think re-acquire) — see [`TFS-RUST_772_Monster_Audit_Verification.md`](TFS-RUST_772_Monster_Audit_Verification.md) §4.3 (N3).

---

## 2. Audit Methodology

### 2.1 Reference sources

All 772 behavior is sourced from `reference/cipsoft-772/tibia-game-master/src/` (decompile outcomes only — never transcribed):

| File | Lines | Purpose |
|------|-------|---------|
| `cract.cc` | 1661 | `TCreature::Execute` (ToDo drain + catch), `Go`, `ToDoStart`, `ToDoYield`, `ToDoWait`, `CalculateDelay` |
| `crnonpl.cc` | 3264 | `TMonster::MovePossible` (planning + execute), `TMonster::KickCreature` (chain-push + kill), `TMonster::IdleStimulus` (chase/attack body + catch) |
| `crmain.cc` | 2176 | `MoveCreatures`, `CreatureMoveStimulus` fan-out |
| `cr.hh` | 1052 | `TToDoEntry`, `ToDoList`, `NextWakeup`, `LockToDo`, `Target` |

### 2.2 Rust files audited

| File | Purpose |
|------|---------|
| `walk/mod.rs` | `drain_todo_queue`, `process_creature_todo`, `on_walk` blocked-step `Err` arm, `internal_move_creature_step`, `move_creature_on_map` |
| `idle_stimulus.rs` | `idle_stimulus`, `request_idle_stimulus`, `monster_exhausted_wait_772`, `execute_creature_todo_action`, `finish_creature_todo_execute`, `run_monster_todo_execute` |
| `creature_todo.rs` | `CreatureAction`, `CreatureTodo`, `creature_todo_yield`, `todo_start_from_action`, `todo_attack_delay_ms`, `idle_enqueue_wait_and_start` |
| `monster_push.rs` | `monster_push_before_step`, `monster_kick_before_step_772`, `monster_kick_creature_772`, `monster_kick_boxes_772` |
| `monster_ai.rs` | `monster_move_possible_planning_772` (the `MovePossible(Execute=false)` port), `monster_idle_skip_idle_melee_chase` |
| `monster_events.rs` | `monster_on_creature_move`, `monster_dispatch_creature_move` |
| `creature/base.rs` | `walk_timer_idle`, `next_wakeup`, `earliest_attack_ms`, `earliest_spell_server_ms` |

### 2.3 Verification approach

- Cross-referenced each Rust function against the exact C++ file:line cited in its doc comment.
- Traced the full blocked-step lifecycle end-to-end on both sides: `Execute` → `Go` → `MovePossible(Execute=true)` → `KickCreature` (chain-push / kill) → `throw EXHAUSTED` / `throw MOVENOTPOSSIBLE` → `Execute` catch → recovery.
- Differential analysis of the `IdleStimulus` catch (`crnonpl.cc:2890-2898`) vs the `Execute` catch (`cract.cc:870-888`) — these are **two different** catch blocks with **different** target-clear semantics.
- Confirmed lesson #31's stated intent ("a normal blocked step still clears-queue+replans on the same beat") is **not** realized for the chase case (see F1).

---

## 3. The 772 C++ Spec — How a Blocked Step Recovers

This is the behavioral specification the Rust port must match. **Do not transcribe; match outcomes.**

### 3.1 `TCreature::Execute` — the ToDo drain loop (`cract.cc:783-898`)

```cpp
void TCreature::Execute(void){
    while(true){
        if(!this->LockToDo || this->IsDead || this->NextWakeup > ServerMilliseconds){
            break;
        }
        if(this->NrToDo <= this->ActToDo){
            this->ToDoClear();
            this->IdleStimulus();          // ← drain → IdleStimulus
            break;
        }
        uint32 Delay = this->CalculateDelay();
        if(Delay > 0){ /* arm NextWakeup, break */ }
        TToDoEntry TD = *this->ToDoList.at(this->ActToDo);
        this->ActToDo += 1;
        try{
            switch(TD.Code){
                case TDGo:  this->Go(TD.Go.x, TD.Go.y, TD.Go.z); break;
                case TDAttack: this->Attack(); break;
                // ...
            }
        }catch(RESULT r){
            bool SnapbackNecessary = (this->ToDoClear() || this->Stop);   // clears todo, NOT Target
            if(r == EXHAUSTED){
                this->ToDoWait(1000);
                this->ToDoStart();
            }else{
                this->ToDoYield();        // MOVENOTPOSSIBLE etc → IdleStimulus NEXT BEAT (1 ms)
            }
            // (player snapback only)
            break;
        }
    }
}
```

### 3.2 `TCreature::Go` — the step attempt (`cract.cc:379-445`)

```cpp
void TCreature::Go(int DestX, int DestY, int DestZ){
    // ... drunk, diagonal, climb checks ...
    if(!this->MovePossible(DestX, DestY, DestZ, true, false)){
        // ... player climb retry ...
        if(this->posz == DestZ){
            throw MOVENOTPOSSIBLE;        // ← hard block (no kick possible)
        }
    }
    // ... step succeeds ...
}
```

### 3.3 `TMonster::MovePossible` — kick/kill side-effects (`crnonpl.cc:2141-2293`)

When `Execute=true` and the destination tile has a creature:

- **Kicker gate**: `State == ATTACKING || PANIC`, `Target != 0`, `RaceData[Race].KickCreatures` (`crnonpl.cc:2194-2204`).
- **Hard blocks** (return `false`, no kick): mover's own `Target` or `Master` (`crnonpl.cc:2212`), `Unpushable` (`crnonpl.cc:2216`), invisible (when no `SeeInvisible`, `crnonpl.cc:2221-2223`), NPC (`crnonpl.cc:2225`), summon-facing-player or `IGNORED_BY_MONSTERS` player (`crnonpl.cc:2229-2233`).
- **Player tile (non-summon, non-IGNORED)**: `this->Target = 0; throw EXHAUSTED;` (`crnonpl.cc:2236-2238`).
- **Pushable monster**: `if(!this->KickCreature(Creature)){ throw EXHAUSTED; }` (`crnonpl.cc:2241-2242`).
- **Kick-and-retry loop** (`crnonpl.cc:2185` `for Attempt 0..100`): after each kick, re-check the destination tile; if still blocked by another creature, kick again. A monster can step through a multi-deep creature wall on the same beat.

### 3.4 `TMonster::KickCreature` — chain-push + kill (`crnonpl.cc:3036-3098`)

```cpp
bool TMonster::KickCreature(TCreature *Creature){
    // ... gates: must be a MONSTER ...
    int OffsetX[4] = { 0,  0, -1,  1};
    int OffsetY[4] = {-1,  1,  0,  0};
    for(int i = 0; i < 4; i += 1){
        DestX = Creature->posx + OffsetX[i];
        DestY = Creature->posy + OffsetY[i];
        if(DestX == this->posx && DestY == this->posy){ continue; }   // skip kicker's tile
        if(Creature->MovePossible(DestX, DestY, DestZ, true, false)   // ← Execute=true (RECURSIVE KICK)
                && !CoordinateFlag(DestX, DestY, DestZ, AVOID)){
            Object Dest = GetMapContainer(DestX, DestY, DestZ);
            ::Move(this->ID, Creature->CrObject, Dest, -1, false, NONE);
            CreatureMoved = true;
            break;
        }
    }
    if(!CreatureMoved){
        GraphicalEffect(Creature->CrObject, EFFECT_BLOCK_HIT);
        Creature->Combat.AddDamageToCombatList(this->ID, Creature->Skills[SKILL_HITPOINTS]->Get());
        Creature->Kill();                // ← boxed-in blocker is killed
    }
    return CreatureMoved;
}
```

**Critical detail:** `Creature->MovePossible(Dest, Execute=true)` (`crnonpl.cc:3066`) — the **blocker's own** `MovePossible` runs in execute mode, so it **recursively kicks** whatever creature is on its escape tile. This is **chain-push**: A pushes B, B pushes C, C moves, B moves into C's spot, A moves into B's spot — all in one beat.

### 3.5 The two catch blocks (do not confuse)

| Catch block | Location | `Target` handling | When it fires |
|-------------|----------|-------------------|---------------|
| **`Execute` catch** | `cract.cc:870-888` | `ToDoClear()` only — **Target preserved** (except player-tile case, which cleared `Target` inside `MovePossible` before throwing) | `Go` throws `EXHAUSTED` (kick-kill or player-tile) or `MOVENOTPOSSIBLE` (hard block) |
| **`IdleStimulus` catch** | `crnonpl.cc:2890-2898` | `this->Target = 0; ToDoClear();` — **Target cleared** | `IdleStimulus`'s own chase/attack body throws (e.g. `ToDoGo` to a now-unreachable target) |

### 3.6 Recovery outcomes (the spec)

| Scenario | C++ outcome | Latency |
|----------|-------------|---------|
| Successful kick/push (incl. chain-push) | Mover steps through, no pause | 0 ms (same beat) |
| Kick-kill (blocker boxed in) | `EXHAUSTED` → `ToDoClear` (no target clear) + `ToDoWait(1000)` + `ToDoStart` → re-engage **same target** after 1 s | 1000 ms |
| Player tile (non-summon) | `Target = 0` (in `MovePossible`) + `EXHAUSTED` → 1 s wait, re-acquire | 1000 ms |
| Hard block (can't kick: target/master/unpushable/NPC/invisible/summon-player/IGNORED) | `MOVENOTPOSSIBLE` → `ToDoClear` + `ToDoYield` → `IdleStimulus` re-runs **next beat** → re-path around blocker | **1 ms** |

The 1 ms `MOVENOTPOSSIBLE` recovery is the key to fluid chasing in dense groups: a monster that can't kick a blocker immediately re-plans around it on the next beat.

---

## 4. Findings

### 4.1 Finding F1 — RESOLVED (superseded by walk-engine unification Phase 1.3)

**Status:** Resolved / superseded by commit `c7b4df4` ("772 walk-engine unification Phase 0 + 1.1-1.3", 2026-07-01).
**Original severity:** Critical
**Original symptom contribution:** "Pause when bumping" — single hard-blocked monster stalls 200 ms – 2000 ms+ instead of re-pathing in 1 ms

> **Re-verification (2026-07-01):** See [`TFS-RUST_772_Monster_Audit_Verification.md`](TFS-RUST_772_Monster_Audit_Verification.md) §2.1. The `retain(!Go)` path described below no longer exists — the `Err` arm now does a full `todo.queue.clear()` + `locked = false` + `request_idle_stimulus`, which is functionally equivalent to the §6.2 recommended fix. The sections below are retained for historical reference.

#### 4.1.1 The Rust blocked-step `Err` arm (`walk/mod.rs:1296-1318`)

When `internal_move_creature_step` returns `Err` (destination still blocked after the kick gate — i.e. a hard block), the 772 monster branch runs:

```rust
// 772: `ToDoClear` + `ToDoYield` on blocked step (`cract.cc:393-414`, `:845-852`).
if self.beat_driven_loop
    && self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
{
    if let Some(k) = self.creatures.get_mut(cid) {
        let base = k.base_mut();
        if base.follow_target.is_some() || base.attack_target.is_some() {
            base.walk_queue.clear();
            base.has_follow_path = false;
            base.force_update_follow_path = true;
            base.todo.queue.retain(|action| !matches!(action, CreatureAction::Go));  // ← keeps Attack
        }
    }
    self.request_idle_stimulus(cid);   // ← bails when Attack still queued
}
```

The comment claims `ToDoClear + ToDoYield`, but the code:
1. Clears `walk_queue` and `has_follow_path`, sets `force_update_follow_path` — **partial** `ToDoClear` (walk queue only).
2. `retain` strips `Go` actions but **keeps `Attack`** (and any `Wait`) — **not** a full `ToDoClear`.
3. Calls `request_idle_stimulus`, which has guards that bail on a non-empty action queue.

#### 4.1.2 `request_idle_stimulus` bails on non-empty queue (`idle_stimulus.rs:105-145`)

```rust
pub(crate) fn request_idle_stimulus(&mut self, cid: CreatureId) {
    if !self.beat_driven_loop { return; }
    if !self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_))) { return; }
    if !self.creatures.get(cid).is_some_and(|k| k.base().walk_timer_idle(self.beat_driven_loop)) { return; }
    if !self.creature_todo_queue_empty(cid) { return; }   // ← Attack present → BAIL, no wakeup armed
    // ... idle-stimulus-last-ms guard, has_wait guard ...
    self.creature_todo_yield(cid);
}
```

The idle combat tail enqueues `Attack` (and often `Go` + `Attack`) — confirmed at `idle_stimulus.rs:1512` (`if self.enqueue_creature_attack(cid) { ... }`). So after `retain` strips `Go`, `[Attack]` remains, `request_idle_stimulus` returns at line 124 **without scheduling any wakeup**.

#### 4.1.3 Control falls through to `finish_creature_todo_execute` defer-attack branch (`idle_stimulus.rs:2338-2368`)

After `execute_creature_todo_action` returns `Some(TodoExecuteKind::Go)`, `run_monster_todo_execute` calls `finish_creature_todo_execute` (`idle_stimulus.rs:2391`). With `walk_queue` empty and `Attack` queued, it hits the `defer_attack_after_go` branch for any ATTACKING monster whose target is >1 away:

```rust
if !self.creature_todo_queue_empty(cid) {
    let defer_attack_after_go = self.creatures.get(cid).is_some_and(|k| {
        let CreatureKind::Monster(m) = k else { return false; };
        if !m.base.todo.has_attack() || m.base.todo.has_go() { return false; }
        if !self.monster_idle_skip_idle_melee_chase(cid) { return false; }   // true for Attacking|Panic
        m.base.attack_target.is_some_and(|aid| {
            self.creatures.get(aid).is_some_and(|t| chebyshev(k.position(), t.position()) > 1)
        })
    });
    if defer_attack_after_go {
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.base.next_wakeup = None;
        }
        let mut delay_ms = self.todo_attack_delay_ms(cid);   // earliest_attack.max(earliest_spell) - server_ms
        if delay_ms == 0 { delay_ms = 200; }
        self.todo_start_from_action(cid, delay_ms);          // ← wakeup at server_ms + (200..2000+)
        return;
    }
    self.run_monster_todo_execute(cid);
}
```

`monster_idle_skip_idle_melee_chase` (`monster_ai.rs:1054-1065`) returns `true` for `Attacking | Panic` — i.e. **any chasing monster**. `todo_attack_delay_ms` (`creature_todo.rs:206-216`) is `earliest_attack_ms.max(earliest_spell_server_ms) - server_ms`. A monster that just swung has `earliest_attack_ms` ~2000 ms in the future, so the wakeup is armed at `server_ms + ~2000`.

#### 4.1.4 Discrepancy vs C++

| | C++ 772 | Rust |
|---|---------|------|
| Hard-block recovery | `ToDoClear()` + `ToDoYield()` → `IdleStimulus` next beat (**1 ms**) | retain-Go + `request_idle_stimulus` bail → `finish_creature_todo_execute` defer `Attack` by `max(attack_delay, 200)` ms (**200 ms – 2000 ms+**) |
| Re-path around blocker | Yes, immediately on next beat | No — deferred until attack cooldown expires, then close-chase-go re-paths |

#### 4.1.5 Observable impact

When a chasing (ATTACKING) monster bumps a **hard-block** creature — most commonly:
- Its own target, when the path's last step lands on the target tile (adjacent-to-player stall)
- An unpushable monster in a corridor
- An NPC blocking a doorway
- The mover's master (summon case)
- An invisible creature the mover can't see

— it freezes for the attack cooldown instead of routing around. With multiple monsters, the back of the pack repeatedly hits the front monster (which is adjacent to you) and stalls one by one.

#### 4.1.6 Note on documented intent

Lesson #31 states the design intent: *"a normal blocked step still clears-queue+replans on the same beat."* The code only clears `walk_queue` and retains non-Go actions — it does **not** clear the action queue, and `request_idle_stimulus` bails when `Attack` is present. So the documented intent is not realized for the chase case.

<ref_file file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/walk/mod.rs" />
<ref_file file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/idle_stimulus.rs" />
<ref_file file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/creature_todo.rs" />

---

### 4.2 Finding F2 — `KickCreature` uses planning gate, not execute gate (no chain-push) (High)

**Severity:** High
**Symptom contribution:** Dense convoys stack/kill/stall where C++ chain-pushes fluidly

#### 4.2.1 The Rust `monster_kick_creature_772` (`monster_push.rs:368-442`)

```rust
fn monster_kick_creature_772(
    &mut self,
    kicker: CreatureId,
    blocker: CreatureId,
    mover_pos: Position,
    _now: Instant,
) -> bool {
    let blocker_pos = match self.creatures.get(blocker) {
        Some(k) => k.position(),
        None => return false,
    };
    for dir in KICK_DIRS_772 {
        let try_pos = blocker_pos.offset(dir);
        if try_pos.x == mover_pos.x && try_pos.y == mover_pos.y && try_pos.z == mover_pos.z {
            continue;
        }
        if let Some(tile) = self.map.get_tile(try_pos) {
            if (tile.body().flags & tilestate::MAGICFIELD) != 0 { continue; }
        }
        // P1-B4: C++ `Creature->MovePossible(Dest, Execute=true)` — the blocker's own 772
        // `MovePossible` gate ... `monster_move_possible_planning_772` is the 772 planning equivalent
        // (no TShortway terrain checks — those are pathfinder-specific).
        let can_occupy = match self.creatures.get(blocker) {
            Some(CreatureKind::Monster(_)) => {
                self.monster_move_possible_planning_772(blocker, try_pos)   // ← Execute=false equivalent
            }
            _ => false,
        };
        if !can_occupy { continue; }
        // Forced relocate — bypasses `tile_query_add_creature` ...
        self.move_creature_on_map(blocker, blocker_pos, try_pos);           // ← forced relocate
        return true;
    }
    // ... kill path ...
    false
}
```

The comment at line 392-395 explicitly acknowledges the substitution: *"monster_move_possible_planning_772 is the 772 planning equivalent."*

#### 4.2.2 `monster_move_possible_planning_772` is `MovePossible(Execute=false)` (`monster_ai.rs:2599-2714`)

For a creature on `try_pos`, when the blocker is ATTACKING with a target + `KickCreatures`, it `continue`s (treats the tile as passable) for pushable monsters (`monster_ai.rs:2689-2696`):

```rust
match other {
    CreatureKind::Monster(m) => {
        if !m.is_pushable() { return false; }
        // C++ `MovePossible` has no summon gate — a summon with KickCreatures
        // plans through pushable monsters like any other kicker (`crnonpl.cc:2202`).
        // P1-A1: the old `!is_summon` gate is dropped.
        continue;   // ← plannable-through, but NO kick
    }
    // ...
}
```

`continue` means "this tile is passable for planning purposes" — it does **not** kick the creature on the escape tile. The recursive kick is missing.

#### 4.2.3 Discrepancy vs C++

C++ `KickCreature` calls `Creature->MovePossible(Dest, Execute=true)` (`crnonpl.cc:3066`). `Execute=true` means the blocker's `MovePossible` **recursively kicks** whatever creature is on the escape tile (`crnonpl.cc:2235-2247`). This is chain-push.

The Rust fix in lesson #77 / P1-B4 replaced the *wrong-era* 1098 `tile_query_add_monster` with the 772 **planning** gate — but C++ uses the 772 **execute** gate. The recursive kick was lost in the substitution.

#### 4.2.4 Two observable consequences

**1. Creature stacking / no chain-push in dense convoys.**

When B's only "free" escape tile has a pushable monster C, planning returns `true`, so `move_creature_on_map` forcibly relocates B **onto C's tile** (`monster_push.rs:410`) — two monsters now share a tile. C++ would have B kick C aside first (chain-push). The stacking corrupts subsequent tile/blocker scans and produces visual glitches and erratic re-pathing.

**2. Spurious kick-kills / EXHAUSTED pauses.**

When B is boxed in by hard blocks on all four sides (walls, unpushable monsters, target/master), planning returns `false` for all dirs → `monster_kick_creature_772` kills B and returns `false` → `monster_kick_before_step_772` returns `Exhausted` (`monster_push.rs:192`) → `monster_exhausted_wait_772` (1 s pause, see F3). C++ would chain-push through the pushable monsters in the convoy and keep moving.

This is the primary cause of the "multiple monsters chasing you, they pause when bumping" symptom in dense groups: where C++ chain-pushes fluidly, Rust either stacks monsters or kills the blocker and stalls the kicker for a second.

<ref_file file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/monster_push.rs" />
<ref_file file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/monster_ai.rs" />
<ref_snippet file="/mnt/storage2/TFS_RUST/reference/cipsoft-772/tibia-game-master/src/crnonpl.cc" lines="3056-3073" />

---

### 4.3 Finding F3 — `monster_exhausted_wait_772` clears target; C++ `Execute` catch preserves it (Medium)

**Severity:** Medium
**Symptom contribution:** Blocked monster drops aggro and re-acquires after 1 s instead of re-engaging the same target

#### 4.3.1 The Rust `monster_exhausted_wait_772` (`idle_stimulus.rs:147-166`)

```rust
/// 772 `EXHAUSTED` recovery — the `TMonster::IdleStimulus` catch block
/// (`crnonpl.cc:2890-2898`): `Target = 0; ToDoClear(); ToDoWait(1000); ToDoStart();`.
///
/// Invoked when a pre-step kick ([`crate::monster_push`]) hit a player tile or had to kill a
/// blocker (`KickCreature` returned `false`) — the mover does **not** step this beat, it drops
/// its target and stalls for a full second instead of clearing-queue and re-planning on the
/// same beat (audit Finding 7).
pub(crate) fn monster_exhausted_wait_772(&mut self, cid: CreatureId) {
    if let Some(k) = self.creatures.get_mut(cid) {
        let base = k.base_mut();
        base.clear_targets();          // ← clears attack_target + follow_target
        base.walk_queue.clear();
        base.has_follow_path = false;
        base.force_update_follow_path = true;
        base.todo.queue.clear();
        base.todo.locked = false;
    }
    trace_creature_todo(self, cid, "monster_exhausted_wait");
    self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);  // 1000 ms
}
```

The doc comment cites `crnonpl.cc:2890-2898` — the **`IdleStimulus` catch block**, which does `Target = 0; ToDoClear(); ToDoWait(1000); ToDoStart()`. But the function is invoked from the **`Execute` catch** path (`walk/mod.rs:1270-1273`), which is a *different* catch block.

#### 4.3.2 The invocation site (`walk/mod.rs:1270-1273`)

```rust
if kick_outcome == crate::monster_push::MonsterKickOutcome::Exhausted {
    self.monster_exhausted_wait_772(cid);
    return;
}
```

This fires when `monster_kick_before_step_772` returns `Exhausted` — i.e. the **`Execute`-path** `EXHAUSTED` (kick-kill or player-tile), thrown from inside `MovePossible(Execute=true)` (`crnonpl.cc:2238` or `crnonpl.cc:2242`) and caught by `Execute`'s catch (`cract.cc:870-888`).

#### 4.3.3 The two C++ catch blocks (revisited)

| Catch block | Location | `Target` handling |
|-------------|----------|-------------------|
| **`Execute` catch** | `cract.cc:870-888` | `ToDoClear()` only — **Target preserved** (except player-tile case, which cleared `Target` inside `MovePossible` at `crnonpl.cc:2237` before throwing) |
| **`IdleStimulus` catch** | `crnonpl.cc:2890-2898` | `this->Target = 0; ToDoClear();` — **Target cleared** |

The `Execute` catch **does not** clear `Target` itself — it relies on the throw site to have cleared it if appropriate. The player-tile throw site (`crnonpl.cc:2236-2238`) clears `Target` before throwing; the kick-kill throw site (`crnonpl.cc:2241-2242`) does **not** clear `Target`.

#### 4.3.4 Discrepancy vs C++

| Case | C++ | Rust |
|------|-----|------|
| `Execute` EXHAUSTED — **kick-kill** (blocker boxed in, killed) | `ToDoClear()` (no target clear) + `ToDoWait(1000)` → **target preserved**, re-engage same target after 1 s | `clear_targets()` + 1000 ms → **target dropped**, re-acquire (possibly different target or sleep) |
| `Execute` EXHAUSTED — **player tile** | `Target = 0` (cleared in `MovePossible` before throw, `crnonpl.cc:2237`) + 1000 ms | `clear_targets()` + 1000 ms → matches (target cleared) |

So the player-tile case matches, but the **kick-kill case does not**: Rust drops the player; C++ keeps the player and re-engages after 1 s.

#### 4.3.5 Observable impact

When a monster kills a boxed-in blocker (F2's spurious kills), it also **drops you as target** and pauses 1 s. In C++ it would keep you targeted and resume the chase after 1 s. Combined with F2, dense convoys in Rust both kill each other *and* lose aggro, amplifying the "they pause and seem to give up" feel.

<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/idle_stimulus.rs" lines="147-166" />
<ref_snippet file="/mnt/storage2/TFS_RUST/reference/cipsoft-772/tibia-game-master/src/cract.cc" lines="870-888" />
<ref_snippet file="/mnt/storage2/TFS_RUST/reference/cipsoft-772/tibia-game-master/src/crnonpl.cc" lines="2235-2247" />

---

## 5. What Already Matches (No Action Needed)

These subsystems are faithful to the 772 reference and are **not** the source of the reported symptoms. Confirmed by lesson #77 (Pass 8 re-audit) and this audit:

| Subsystem | Status | C++ Reference | Rust File |
|-----------|--------|---------------|-----------|
| Kick/kill structure + full-HP damage attribution | ✅ Exact | `crnonpl.cc:3076-3080` | `monster_push.rs:414-441` |
| Fixed N,S,W,E offset order, no RNG | ✅ Exact | `crnonpl.cc:3057-3058` | `KICK_DIRS_772` (`monster_push.rs`) |
| `CanKickBoxes` master-chain inheritance | ✅ Exact | `crnonpl.cc:2984-2992` | `monster_push.rs:76-94` |
| Kick-and-retry loop on mover's dest tile | ✅ Exact | `crnonpl.cc:2185` `for Attempt 0..100` | `monster_push.rs:140-198` |
| Kicker gate (ATTACKING/PANIC + target + KickCreatures) | ✅ Exact | `crnonpl.cc:2194-2204` | `monster_push.rs:130-132` |
| Hard-block identification (target/master/unpushable/NPC/invisible/summon-player/IGNORED) | ✅ Exact | `crnonpl.cc:2212-2233` | `monster_push.rs:161-187` |
| `EXHAUSTED` 1000 ms wait duration | ✅ Exact | `crnonpl.cc:2894` | `MONSTER_IDLE_WAIT_MS` (`creature_todo.rs:18`) |
| `ToDoStart` +1 clamp (anti-re-entrancy) | ✅ Exact | `cract.cc:1016` | `creature_todo.rs:201` |
| `ToDoYield` = `ToDoWait(0)` + `ToDoStart` → 1 ms wakeup | ✅ Exact | `cract.cc:1026-1031` | `creature_todo.rs:360-378` |
| `MoveCreatures` drain (`<= server_ms`, all due) | ✅ Exact | `crmain.cc:1144-1158` | `walk/mod.rs:337-374` |

---

## 6. Fix Plan

### 6.1 Priority order

| Priority | Finding | Effort | Impact | Risk |
|----------|---------|--------|--------|------|
| ~~P0~~ | ~~F1 — Hard-block `Err` path: full `ToDoClear` + direct `creature_todo_yield`~~ | ~~Small~~ | **Resolved** by `c7b4df4` (Phase 1.3) — full clear + yield landed. Residual: indirect yield via `request_idle_stimulus` (see §6.2, deferred to Step 5 of the verification doc) | Low |
| **P1** | F2 — `KickCreature` recursive chain-push via execute-mode `MovePossible` | Medium | Eliminates stacking + spurious kills in dense convoys | Medium (recursive kick; needs cycle guard) |
| **P2** | F3 — Split EXHAUSTED semantics (kick-kill preserves target, player-tile clears) | Small | Corrects aggro drop on kick-kill | Low (localized to `monster_exhausted_wait_772`) |

### 6.2 Fix F1 — Hard-block `Err` path mirrors C++ `MOVENOTPOSSIBLE` recovery

> **Status (2026-07-01):** ~90% landed by commit `c7b4df4` (Phase 1.3). The `Err` arm now does a full `todo.queue.clear()` + `locked = false` + `request_idle_stimulus` (was: `retain(!Go)` + `request_idle_stimulus`). The `request_idle_stimulus` guards all pass in the hard-block scenario (queue empty, `walk_timer_idle` true, no dedup, no `Wait`), so the monster re-runs `IdleStimulus` on the next beat (~1 ms logical). The one residual deviation from the recommendation below: the landed code calls `request_idle_stimulus` (indirect yield through guards) instead of a direct `creature_todo_yield`. This is functionally equivalent in the traced scenario but adds guard coupling. Switching to a direct `creature_todo_yield` is deferred to Step 5 of [`TFS-RUST_772_Monster_Audit_Verification.md`](TFS-RUST_772_Monster_Audit_Verification.md). The fix sketch below is retained for reference.

**Goal:** A hard-blocked `Go` (no kick possible) must re-run `IdleStimulus` on the **next beat (1 ms)**, matching C++ `Execute` catch `MOVENOTPOSSIBLE` → `ToDoClear` + `ToDoYield` (`cract.cc:875-877`).

#### 6.2.1 C++ reference

```cpp
// cract.cc:870-888 (Execute catch)
}catch(RESULT r){
    bool SnapbackNecessary = (this->ToDoClear() || this->Stop);   // FULL ToDoClear
    if(r == EXHAUSTED){
        this->ToDoWait(1000);
        this->ToDoStart();
    }else{
        this->ToDoYield();        // ← MOVENOTPOSSIBLE path: ToDoWait(0) + ToDoStart → 1 ms wakeup
    }
    // ...
    break;
}
```

`ToDoYield` (`cract.cc:1026-1031`):
```cpp
void TCreature::ToDoYield(void){
    if(!this->LockToDo){
        this->ToDoWait(0);
        this->ToDoStart();
    }
}
```

`ToDoStart` clamps `Delay < 1` to `1` (`cract.cc:1016`) → wakeup at `server_ms + 1` (next beat). `IdleStimulus` runs when the todo list drains on that wakeup (`cract.cc:789-792`).

#### 6.2.2 Rust fix

In the `Err` arm of `on_walk` (`walk/mod.rs:1296-1318`), replace the partial-clear + `request_idle_stimulus` with a full `ToDoClear` + direct `creature_todo_yield`:

```rust
// 772: `ToDoClear` + `ToDoYield` on blocked step — mirrors C++ `Execute` catch
// MOVENOTPOSSIBLE path (`cract.cc:870-877`). Full queue clear + 1 ms wakeup →
// IdleStimulus re-paths around the hard blocker on the next beat.
if self.beat_driven_loop
    && self.creatures.get(cid).is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
{
    if let Some(k) = self.creatures.get_mut(cid) {
        let base = k.base_mut();
        if base.follow_target.is_some() || base.attack_target.is_some() {
            base.walk_queue.clear();
            base.has_follow_path = false;
            base.force_update_follow_path = true;
            base.todo.queue.clear();        // ← FULL ToDoClear (was: retain !Go)
            base.todo.locked = false;       // ← unlock for ToDoYield
        }
    }
    // Direct ToDoYield — bypasses request_idle_stimulus guards (which bail on
    // non-empty queue). C++ ToDoYield is unconditional when !LockToDo
    // (`cract.cc:1026-1031`).
    self.creature_todo_yield(cid);          // ← was: request_idle_stimulus(cid)
}
```

`creature_todo_yield` (`creature_todo.rs:360-378`) already implements `ToDoWait(0)` + `ToDoStart` with the +1 clamp, arming wakeup at `server_ms + 1`. On that wakeup, `process_creature_todo` (`walk/mod.rs:377`) drains the `Wait(0)`, finds the queue empty, and calls `maybe_idle_stimulus_after_go_complete` → `monster_idle_stimulus` (`idle_stimulus.rs:2376-2378`), which re-paths with `force_update_follow_path = true`.

#### 6.2.3 Why this is safe

- `creature_todo_yield` has its own `locked` guard (`creature_todo.rs:364-370`) — if the todo is still locked (shouldn't be after `base.todo.locked = false`), it bails. We clear `locked` first.
- The +1 clamp (`creature_todo.rs:201`) guarantees the wakeup is strictly future — no same-beat re-entry (audit Finding 17).
- `force_update_follow_path = true` ensures the re-plan recomputes the path rather than re-using the blocked one.
- Target is **preserved** (no `clear_targets`), matching C++ `Execute` catch `MOVENOTPOSSIBLE` (which does not clear `Target`).

#### 6.2.4 Test cases to add (failing first)

1. `f1_hard_block_reruns_idle_next_beat`: A chasing monster whose only path step lands on its own target (adjacent, hard block) → assert `next_wakeup == server_ms + 1` and `IdleStimulus` re-runs on the next beat (re-path around).
2. `f1_hard_block_preserves_target`: Same scenario → assert `attack_target` and `follow_target` are unchanged after the blocked step.
3. `f1_hard_block_clears_action_queue`: Same scenario → assert `todo.queue.is_empty()` after the blocked step (was: `[Attack]` retained).
4. `f1_hard_block_unpushable_monster`: A chasing monster blocked by an unpushable monster → assert re-path within 1 beat (was: 200 ms – 2000 ms+ defer).

<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/walk/mod.rs" lines="1296-1318" />
<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/creature_todo.rs" lines="359-378" />

---

### 6.3 Fix F2 — `KickCreature` recursive chain-push via execute-mode `MovePossible`

**Goal:** When validating a blocker's escape tile, if the tile has a pushable monster, **recursively kick it** (chain-push) instead of treating the tile as plannable-through. This matches C++ `KickCreature` calling `Creature->MovePossible(Dest, Execute=true)` (`crnonpl.cc:3066`).

#### 6.3.1 C++ reference

```cpp
// crnonpl.cc:3036-3098 (TMonster::KickCreature)
for(int i = 0; i < 4; i += 1){
    DestX = Creature->posx + OffsetX[i];
    DestY = Creature->posy + OffsetY[i];
    if(DestX == this->posx && DestY == this->posy){ continue; }
    if(Creature->MovePossible(DestX, DestY, DestZ, true, false)   // ← Execute=true
            && !CoordinateFlag(DestX, DestY, DestZ, AVOID)){
        Object Dest = GetMapContainer(DestX, DestY, DestZ);
        ::Move(this->ID, Creature->CrObject, Dest, -1, false, NONE);
        CreatureMoved = true;
        break;
    }
}
```

`Creature->MovePossible(Dest, Execute=true)` runs the **blocker's own** `MovePossible` in execute mode, which kicks whatever creature is on the escape tile (`crnonpl.cc:2235-2247`). This is the recursive chain-push.

#### 6.3.2 Rust fix

Introduce an **execute-mode** `MovePossible` for the kick dest validation that recursively kicks creatures on the escape tile. The simplest correct shape:

```rust
/// 772 `TMonster::MovePossible(Execute=true)` for `KickCreature` dest validation
/// (`crnonpl.cc:3066`). Unlike [`monster_move_possible_planning_772`] (Execute=false),
/// this recursively kicks pushable creatures on the escape tile (chain-push) before
/// declaring it passable. Returns `true` if the blocker can occupy `try_pos` (after
/// any chain-kick side-effects), `false` on a hard block.
fn monster_move_possible_execute_for_kick_772(
    &mut self,
    blocker: CreatureId,
    try_pos: Position,
    kicker_pos: Position,
    now: Instant,
) -> bool {
    // Reuse the planning gate for non-creature blocks (leash, PZ, house, items, terrain).
    // The creature branch is where execute-mode diverges.
    if !self.monster_move_possible_planning_772(blocker, try_pos) {
        return false;
    }
    // Planning returned true — but if a pushable creature is on try_pos, planning
    // treated it as plannable-through. Execute-mode must KICK it (chain-push).
    let blockers_on_try: Vec<CreatureId> = self
        .map
        .get_tile(try_pos)
        .map(|t| t.body().creatures.iter().copied().filter(|&c| c != blocker).collect())
        .unwrap_or_default();
    for other in blockers_on_try {
        // Hard blocks (target/master/unpushable/NPC/invisible/summon-player/IGNORED)
        // already returned false via planning. Only pushable monsters reach here.
        // Recursively kick — C++ `Creature->MovePossible(Execute=true)` (`crnonpl.cc:3066`).
        if !self.monster_kick_creature_772(blocker, other, kicker_pos, now) {
            return false;   // kick failed (kill or no escape) → tile not passable
        }
    }
    true
}
```

Then update `monster_kick_creature_772` to call the execute-mode gate:

```rust
// monster_push.rs:396-403 (replace the planning call)
let can_occupy = match self.creatures.get(blocker) {
    Some(CreatureKind::Monster(_)) => {
        // C++ `Creature->MovePossible(Dest, Execute=true)` (`crnonpl.cc:3066`) —
        // recursively kicks creatures on the escape tile (chain-push). Was: planning
        // gate (Execute=false), which skipped the recursive kick and caused stacking
        // + spurious kills in dense convoys (audit F2).
        self.monster_move_possible_execute_for_kick_772(blocker, try_pos, mover_pos, _now)
    }
    _ => false,
};
```

#### 6.3.3 Cycle guard

The recursive kick can cycle (A pushes B, B pushes A). C++ has the `for Attempt 0..100` loop on the *mover's* dest (`crnonpl.cc:2185`) as a bound, but the recursive `KickCreature` has no explicit cycle guard — it relies on the fixed N,S,W,E offset order and the `skip kicker's tile` check (`crnonpl.cc:3062-3064`) to terminate. Rust should add a visited-set or depth bound (e.g. `MAX_KICK_DEPTH = 8`) to prevent infinite recursion in pathological configurations:

```rust
fn monster_kick_creature_772_inner(
    &mut self,
    kicker: CreatureId,
    blocker: CreatureId,
    mover_pos: Position,
    now: Instant,
    depth: u8,
) -> bool {
    if depth >= MAX_KICK_DEPTH { return false; }   // cycle guard
    // ... existing body, but recursive call passes depth + 1 ...
}
```

#### 6.3.4 Why this is the bigger change

- The recursive kick mutates the map (moves creatures) during what was previously a read-only planning call. The borrow checker will require careful snapshotting (collect blocker IDs first, then mutate) — the existing `monster_kick_before_step_772` already does this pattern (`monster_push.rs:141-152`).
- The cycle guard is new code not present in C++ (C++ relies on offset order + skip-kicker); Rust should be explicit.
- Tests must cover: chain-push (A→B→C), cycle (A↔B with no escape → both killed or one killed), depth limit, and the existing single-kick cases (regression).

#### 6.3.5 Test cases to add (failing first)

1. `f2_chain_push_three_monsters`: A→B→C in a line, C's escape tile is free → assert A steps through, B relocates to C's spot, C relocates to the free tile (one beat, no stacking).
2. `f2_chain_push_no_stacking`: A→B where B's only escape has a pushable C → assert B and C do **not** share a tile after the kick (was: stacking).
3. `f2_chain_push_boxed_in_kills`: A→B where B is boxed in by hard blocks on all four sides → assert B is killed (full-HP damage attributed to A) and A returns `Exhausted` (regression of existing behavior).
4. `f2_chain_push_cycle_guard`: A↔B with no escape for either → assert no infinite recursion (depth limit hit, both return `false`).
5. `f2_dense_convoy_fluid`: 5 monsters in a line chasing a player through a 1-wide corridor → assert all 5 advance one tile per beat (was: stacking + stalls).

<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/monster_push.rs" lines="380-412" />
<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/monster_ai.rs" lines="2660-2697" />
<ref_snippet file="/mnt/storage2/TFS_RUST/reference/cipsoft-772/tibia-game-master/src/crnonpl.cc" lines="3056-3098" />

---

### 6.4 Fix F3 — Split EXHAUSTED semantics (kill-kill preserves target, player-tile clears)

**Goal:** The kick-kill `Exhausted` outcome should run the **`Execute` catch** recovery (`ToDoClear` + `ToDoWait(1000)` + `ToDoStart`, **no `clear_targets`**). Only the **player-tile** `Exhausted` outcome should clear the target (matching `crnonpl.cc:2237`).

#### 6.4.1 C++ reference

```cpp
// cract.cc:870-888 (Execute catch — does NOT clear Target itself)
}catch(RESULT r){
    bool SnapbackNecessary = (this->ToDoClear() || this->Stop);   // clears todo, NOT Target
    if(r == EXHAUSTED){
        this->ToDoWait(1000);
        this->ToDoStart();
    }else{
        this->ToDoYield();
    }
    // ...
}

// crnonpl.cc:2235-2243 (MovePossible Execute=true throw sites)
if(Execute){
    if(Creature->Type == PLAYER){
        this->Target = 0;            // ← player-tile: Target cleared HERE, before throw
        throw EXHAUSTED;
    }
    if(!this->KickCreature(Creature)){
        throw EXHAUSTED;             // ← kick-kill: Target NOT cleared
    }
    break;
}
```

The `Execute` catch relies on the throw site to have cleared `Target` if appropriate. The player-tile site clears it; the kick-kill site does not.

#### 6.4.2 Rust fix

Introduce a second `MonsterKickOutcome` variant to distinguish the two EXHAUSTED cases, or pass a target-clear flag through. The variant approach is cleaner:

```rust
// monster_push.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterKickOutcome {
    Proceed,
    Exhausted,           // ← kick-kill: Target preserved (C++ Execute catch)
    ExhaustedDropTarget, // ← player-tile: Target cleared (C++ crnonpl.cc:2237)
}
```

Update `monster_kick_before_step_772` to return the appropriate variant:

```rust
// monster_push.rs:184 (player-tile EXHAUSTED)
Some(CreatureKind::Player(_)) => return MonsterKickOutcome::ExhaustedDropTarget,

// monster_push.rs:191-193 (kick-kill EXHAUSTED — monster_kick_creature_772 returned false)
Some(CreatureKind::Monster(_)) => {
    if !self.monster_kick_creature_772(mover, blocker, mover_pos, now) {
        return MonsterKickOutcome::Exhausted;   // ← Target preserved
    }
    // ...
}
```

Split `monster_exhausted_wait_772` into two paths (or add a `clear_target: bool` parameter):

```rust
// idle_stimulus.rs
/// 772 `Execute` catch EXHAUSTED recovery (`cract.cc:870-877`).
/// `clear_target` mirrors the C++ throw-site distinction:
/// - player-tile (`crnonpl.cc:2237`): `Target = 0` before throw → `clear_target = true`
/// - kick-kill (`crnonpl.cc:2241-2242`): Target NOT cleared → `clear_target = false`
/// The `Execute` catch itself does NOT clear Target — it relies on the throw site.
pub(crate) fn monster_exhausted_wait_772(&mut self, cid: CreatureId, clear_target: bool) {
    if let Some(k) = self.creatures.get_mut(cid) {
        let base = k.base_mut();
        if clear_target {
            base.clear_targets();          // ← player-tile only
        }
        base.walk_queue.clear();
        base.has_follow_path = false;
        base.force_update_follow_path = true;
        base.todo.queue.clear();
        base.todo.locked = false;
    }
    trace_creature_todo(self, cid, "monster_exhausted_wait");
    self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
}
```

Update the invocation site (`walk/mod.rs:1270-1273`):

```rust
match kick_outcome {
    crate::monster_push::MonsterKickOutcome::Exhausted => {
        // Kick-kill: Target preserved (C++ Execute catch + crnonpl.cc:2241-2242).
        self.monster_exhausted_wait_772(cid, false);
    }
    crate::monster_push::MonsterKickOutcome::ExhaustedDropTarget => {
        // Player-tile: Target cleared (C++ crnonpl.cc:2237).
        self.monster_exhausted_wait_772(cid, true);
    }
    crate::monster_push::MonsterKickOutcome::Proceed => { /* fall through to step */ }
}
```

#### 6.4.3 Update the doc comment

The current doc comment cites `crnonpl.cc:2890-2898` (the `IdleStimulus` catch), which is the wrong catch block. Update to cite `cract.cc:870-877` (the `Execute` catch) with the throw-site distinction:

```rust
/// 772 `Execute` catch EXHAUSTED recovery (`cract.cc:870-877`):
/// `ToDoClear() + ToDoWait(1000) + ToDoStart()`. The `Execute` catch does NOT clear
/// `Target` itself — it relies on the throw site:
/// - player-tile (`crnonpl.cc:2236-2238`): `Target = 0` before `throw EXHAUSTED`
/// - kick-kill (`crnonpl.cc:2241-2242`): `KickCreature` returned false → `throw EXHAUSTED`
///   (Target NOT cleared)
///
/// `clear_target` mirrors this distinction. The previous implementation unconditionally
/// cleared the target, citing the `IdleStimulus` catch (`crnonpl.cc:2890-2898`) — wrong
/// catch block (audit F3).
```

#### 6.4.4 Test cases to add (failing first)

1. `f3_kick_kill_preserves_target`: A monster kick-kills a boxed-in blocker → assert `attack_target` and `follow_target` are unchanged after the 1 s wait (was: cleared).
2. `f3_player_tile_clears_target`: A monster steps onto a player tile → assert `attack_target` and `follow_target` are cleared after the 1 s wait (regression of existing behavior).
3. `f3_kick_kill_reengages_same_target`: A monster kick-kills a blocker, waits 1 s, then re-engages the **same** player (target was preserved) → assert chase resumes against the original target.

<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/idle_stimulus.rs" lines="147-166" />
<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/walk/mod.rs" lines="1270-1273" />
<ref_snippet file="/mnt/storage2/TFS_RUST/reference/cipsoft-772/tibia-game-master/src/crnonpl.cc" lines="2235-2247" />

---

## 7. Implementation Order & Verification

### 7.1 Suggested commit sequence

1. **F1** — ~~P0, smallest blast radius~~ **Resolved** by `c7b4df4` (Phase 1.3, 2026-07-01). No code needed. Residual: indirect yield hardening deferred to Step 5 of the verification doc.
2. **F3** (P2, small) — `monster_push.rs` + `idle_stimulus.rs` + `walk/mod.rs`: split `MonsterKickOutcome`, add `clear_target` param. Failing tests first, then fix.
3. **F2** (P1, medium) — `monster_push.rs` + `monster_ai.rs`: introduce `monster_move_possible_execute_for_kick_772` with recursive chain-push + cycle guard. Failing tests first, then fix. This is the largest change and should land last so F3 is stable.

### 7.2 Verification per fix

- **F1:** Run the existing chase parity harness (`scripts/run_kite_scenario.py --synthetic` per lesson #60) — the hard-block re-path latency should drop from 200 ms – 2000 ms+ to 1 ms.
- **F2:** Add a dense-convoy scenario (5 monsters, 1-wide corridor) to the harness — assert all 5 advance one tile per beat with no stacking.
- **F3:** Assert target preservation on kick-kill via unit test (`f3_kick_kill_preserves_target`).
- **All:** `rtk cargo test -p tfs-rust-core` must pass (457+ tests as of RC1+RC2).

### 7.3 Lessons to update

After implementation, append to `tasks/lessons.md`:
- F1: "Hard-block `Err` arm must full-clear + `creature_todo_yield`, not retain-Go + `request_idle_stimulus` — the latter bails on non-empty queue and defers by attack cooldown."
- F2: "`KickCreature` dest validation must use execute-mode `MovePossible` (recursive chain-push), not planning gate — planning skips the recursive kick and causes stacking + spurious kills."
- F3: "`monster_exhausted_wait_772` must distinguish kick-kill (Target preserved) from player-tile (Target cleared) — the `Execute` catch does not clear Target itself; the throw site does (`crnonpl.cc:2237` vs `crnonpl.cc:2241-2242`)."

---

## 8. Cross-References

- [`TFS-RUST_772_Monster_AI_Transition_Audit.md`](TFS-RUST_772_Monster_AI_Transition_Audit.md) — RC1 (think cadence) + RC2 (idle trailing wait). **Different symptom** (idle transitions / sleep stalls), **different seam** (think sweep + idle tail). F1-F3 here are collision/blocked-step recovery and do not overlap.
- `tasks/lessons.md` #31 — push/kick split architecture and EXHAUSTED semantics (the stated intent F1 realizes).
- `tasks/lessons.md` #77 — Pass 8 re-audit: 10 push/collision divergences fixed (kick-and-retry loop, summon gate, player tile, IGNORED, invisibility, KickCreature dest validation). F2 here is the **execute vs planning** distinction that #77's P1-B4 substituted (with acknowledgment).
- `tasks/lessons.md` #80 — `CreatureAction::Rotate` + atomic `Execute` drain (the `finish_creature_todo_execute` → `run_monster_todo_execute` tail recursion that F1's fix flows through).
- [`IDLE_STIMULUS.md`](IDLE_STIMULUS.md) — idle stimulus architecture.
- [`GAME_LOOP_ARCHITECTURE.md`](GAME_LOOP_ARCHITECTURE.md) — beat timer, ToDoQueue, lag guard.
