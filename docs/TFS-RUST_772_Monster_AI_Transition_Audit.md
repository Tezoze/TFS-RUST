# TFS-RUST 772 — Monster AI Transition & Pause Audit

**Date:** 2026-06-28  
**Scope:** `crates/tfs-rust-core` monster AI: ToDoQueue scheduler, idle stimulus, server beat, creature packets, map Z-level targeting  
**Symptom:** "Monsters do what they are meant to, but the transition doesn't feel right and there are occasional pauses."  
**Reference:** `reference/cipsoft-772/tibia-game-master/src/` (decompile outcomes only — Rust is idiomatic, not transcribed)  
**Related:** [`TFS-RUST_772_Monster_AI_1098_Leak_Audit.md`](TFS-RUST_772_Monster_AI_1098_Leak_Audit.md), [`GAME_LOOP_ARCHITECTURE.md`](GAME_LOOP_ARCHITECTURE.md), [`IDLE_STIMULUS.md`](IDLE_STIMULUS.md)

---

## 1. Executive Summary

The ToDoQueue binary heap, beat cadence (200 ms), walk quantization, lag guard, packet opcodes, and target validation all faithfully match the 772 reference. The reported "transitions don't feel right and occasional pauses" were caused by **two root-cause bugs** and **two compounding bugs**, all in the idle/think layer — not in the scheduler or packet layer.

| # | Severity | Finding | Subsystem | Status |
|---|----------|---------|-----------|--------|
| RC1 | **Critical** | `process_creatures_772` runs AI think — C++ `ProcessCreatures` is regen-only | think cadence | **Fixed** (2026-06-28) |
| RC2 | **Critical** | Missing unconditional trailing `ToDoWait(1000)` when idle produces no action | idle stimulus | **Fixed** (2026-06-28) |
| B3 | High | Inverted `is_summon` check in `monster_sleep_wake_on_creature_move` | sleep wake | Open |
| B4 | Medium | `monster_update_idle_status` sends monsters to sleep from the think sweep | sleep transition | **Resolved by RC1** |
| M5 | Low | `creature_can_see` Z-level split stricter than 772 `IsVisible` | target selection | Open |

**Bottom line:** RC1 + RC2 are fixed (457 tests pass). These two fixes eliminate the bulk of the "transitions don't feel right" and "occasional pauses" symptoms. B3 is a one-line fix that corrects erratic sleep/wake behavior. B4 is resolved by RC1. M5 is a minor target-selection edge case.

---

## 2. Audit Methodology

### 2.1 Reference sources

All 772 behavior is sourced from `reference/cipsoft-772/tibia-game-master/src/` (decompile outcomes only — never transcribed):

| File | Lines | Purpose |
|------|-------|---------|
| `cract.cc` | 1661 | `TCreature::Execute`, `CalculateDelay`, `ToDoStart`, `ToDoYield`, `Go`, `Rotate` |
| `crnonpl.cc` | 3264 | `TMonster::IdleStimulus`, `DamageStimulus`, `CreatureMoveStimulus`, target selection, idle wandering |
| `crmain.cc` | 2176 | `MoveCreatures`, `ProcessCreatures`, `CreatureMoveStimulus` fan-out |
| `cr.hh` | 1052 | `TToDoEntry` struct, `ToDoList` fields, `NextWakeup`, `LockToDo` |
| `main.cc` | — | `AdvanceGame`, `LaunchGame`, beat timer, lag guard |
| `sending.cc` | 1777 | `SendMoveCreature`, `SendAddField`, `SendDeleteField`, `SendChangeField`, `SendMapObject` |
| `connections.cc` | — | `TConnection::IsVisible` (client viewport) |
| `containers.hh` | — | `priority_queue<K,T>` (verbatim heap) |
| `enums.hh` | — | `STATE` enum (SLEEPING/IDLE/UNDERATTACK/PANIC/…) |

### 2.2 Rust files audited

| File | Purpose |
|------|---------|
| `creature_todo.rs` | `CreatureAction`, `CreatureTodo`, `todo_start_from_action`, `creature_todo_yield` |
| `todo_queue.rs` | Verbatim CipSoft `priority_queue` port (1-indexed binary heap) |
| `game_loop.rs` | `run_game_loop_772` (beat timer, `MissedTickBehavior::Burst`) |
| `game_world_tick.rs` | `advance_beat_772`, lag guard, subsystem dispatch |
| `subsystem_counters_772.rs` | Staggered counters (1750/1500/1250/1000 ms thresholds) |
| `walk/mod.rs` | `drain_todo_queue`, `process_creature_todo`, `schedule_creature_wakeup`, `todo_start_go_delay` |
| `walk/walk_timing.rs` | `linear_go_step_duration_ms`, beat quantization |
| `creature_think.rs` | `process_creatures_772`, `check_creatures`, `monster_on_think` |
| `monster_ai.rs` | `monster_native_on_think`, `monster_combat_reschedule_if_stalled` |
| `idle_stimulus.rs` | `idle_stimulus`, `monster_idle_stimulus_inner`, `monster_sleep_wake_on_creature_move`, walk branches |
| `monster_events.rs` | `monster_on_creature_move`, `CreatureMoveStimulus` fan-out |
| `monster_targets.rs` | `monster_update_idle_status`, `monster_set_idle` |
| `game_world_spectators.rs` | `creature_can_see`, `can_see_position` |

### 2.3 Verification approach

- Cross-referenced each Rust function against the exact C++ file:line cited in its doc comment.
- Confirmed struct field names, control flow, magic numbers, and packet opcodes against the decompile.
- Traced the full idle→sleep→wake lifecycle end-to-end on both sides.
- Differential analysis of the "no target, roam failed" stall path.

---

## 3. What Matches (No Action Needed)

These subsystems are faithful to the 772 reference and are **not** the source of the reported symptoms.

| Subsystem | Status | C++ Reference | Rust File |
|-----------|--------|---------------|-----------|
| ToDoQueue binary heap (structural tie order) | ✅ Exact | `containers.hh:150–227` | `todo_queue.rs` |
| Beat cadence 200 ms (configurable) | ✅ Exact | `config.cc:102`, `main.cc:162–164` | `formulas.rs:279` |
| Walk quantization to beat boundaries (`ceil(Delay/Beat)*Beat`) | ✅ Exact | `cract.cc:1530–1538` | `walk_timing.rs:169–181` |
| `ToDoStart` +1 clamp (anti-re-entrancy) | ✅ Exact | `cract.cc:1016` | `creature_todo.rs:201` |
| `MoveCreatures` drain (`<= server_ms`, all due, no per-tick cap) | ✅ Exact | `crmain.cc:1144–1158` | `walk/mod.rs:337–374` |
| Lag guard at 1000 ms (skip `MoveCreatures`) | ✅ Exact | `main.cc:444–453` | `game_world_tick.rs:77–89` |
| Subsystem counter thresholds (1750/1500/1250/1000 ms) | ✅ Exact | `main.cc:320–340` | `subsystem_counters_772.rs:24–32` |
| `MissedTickBehavior::Burst` for beat coalescing | ✅ Exact | `main.cc:493–496` (`SigAlarmCounter` + `timer_getoverrun`) | `game_loop.rs:589` |
| Rotate-then-move in same beat (no queued `TDRotate`) | ✅ Matches C++ | `crnonpl.cc:2872–2873` | `idle_stimulus.rs:1575–1620` |
| Packet 0x6D `MOVE_CREATURE` (single per step, from+to) | ✅ Exact | `sending.cc:658–694` | `walk/mod.rs` (codec) |
| Packet 0x6B `CHANGE_FIELD` (turn, with creature data) | ✅ Exact | `sending.cc:615–635` | `walk/mod.rs:268–324` |
| Packet 0x6A `ADD_FIELD` / 0x6C `DELETE_FIELD` | ✅ Exact | `sending.cc:597–656` | codec |
| No step batching (per-step packets) | ✅ Exact | `cract.cc:783–898` | — |
| Target validation (Z, range>10, PZ, house, invisible, `LoseTarget`) | ✅ Exact | `crnonpl.cc:2418–2435` | `idle_stimulus.rs:474–520` |
| Target selection strategy (NEAREST/HEALTH/DAMAGE/RANDOM) | ✅ Exact | `crnonpl.cc:2470–2544` | `idle_stimulus.rs:525–658` |
| Zero reaction time on wake (`ToDoYield` = `ToDoWait(0)` → 1 ms) | ✅ Exact | `cract.cc:1026–1031` | `creature_todo.rs:360–378` |
| 1000 ms standard action delay (`MONSTER_IDLE_WAIT_MS`) | ✅ Exact | `crnonpl.cc:2929,2938` | `creature_todo.rs:18` |
| `EXHAUSTED` 1000 ms penalty | ✅ Exact | `cract.cc:872–874` | `idle_stimulus.rs:154–166` |
| `CreatureMoveStimulus` close-chase re-arm (head `TDAttack`) | ✅ Exact | `crmain.cc:888–961` | `monster_events.rs:422–496` |
| Z-level target clear deferred to idle (not on creature move) | ✅ Exact | `crmain.cc:920` (no Z-clear) | `monster_events.rs:200–217` |

---

## 4. Findings

### 4.1 Root Cause #1 — `process_creatures_772` runs AI think; C++ `ProcessCreatures` is regen-only

**Severity:** Critical  
**Status:** **Fixed** (2026-06-28)  
**Symptom contribution:** "Transitions don't feel right" (abrupt chase→idle→sleep, premature target loss)

#### C++ 772 `ProcessCreatures` (`crmain.cc:1075–1138`)

The 772 `ProcessCreatures` is a **regeneration-only** sweep. It does:

1. HP/mana regen based on `SKILL_FED` interval (`RoundNr % RegenInterval == 0`) — `crmain.cc:1089–1097`
2. Player `CheckState()` and logout marks — `crmain.cc:1100–1110`
3. Death safety checks (HP ≤ 0 but not dead → `Death()`) — `crmain.cc:1113–1117`
4. Logout processing — `crmain.cc:1119–1135`

It does **NOT** call `onThink`, `IdleStimulus`, target validation, or idle status updates. Monster AI is driven **entirely** by:

- `IdleStimulus` — when the ToDoQueue drains (`cract.cc:789–791`)
- `CreatureMoveStimulus` — when a creature moves (`crmain.cc:888`)
- `DamageStimulus` — when damage is taken (`crnonpl.cc:2278`)

#### Rust port `process_creatures_772` (`creature_think.rs:108–137`)

```rust
pub fn process_creatures_772(&mut self) {
    let interval_ms = EVENT_CREATURE_THINK_INTERVAL_MS;
    let ids: Vec<CreatureId> = self.creatures.iter()
        .filter(|(_, k)| matches!(k, CreatureKind::Monster(_) | CreatureKind::Npc(_))
            && k.base().think_check_bucket.is_some())
        .map(|(id, _)| id)
        .collect();

    for cid in ids {
        if !self.creature_alive_for_think(cid) { continue; }
        match self.creatures.get(cid) {
            Some(CreatureKind::Monster(_)) => {
                self.monster_on_think(cid, interval_ms);  // ← WRONG
                if self.creature_alive_for_think(cid) && !self.beat_driven_loop {
                    self.creature_on_attacking(cid, interval_ms);
                }
            }
            Some(CreatureKind::Npc(_)) => self.npc_on_think(cid, interval_ms),
            _ => continue,
        }
    }
}
```

`monster_on_think` (`creature_think.rs:243–246`) calls:
1. `creature_on_think` → **clears follow/attack targets** that are no longer visible (`creature_think.rs:155–169`). In C++ 772, this target clearing only happens inside `IdleStimulus` (`crnonpl.cc:2418–2435`), not on a 1 Hz timer.
2. `monster_native_on_think` → calls `monster_update_idle_status` (`monster_ai.rs:766`) → **sends monsters to `Sleeping`** when `opponent_ids.is_empty()` (`monster_targets.rs:353–358`). In C++ 772, sleep transition only happens inside `IdleStimulus` (`crnonpl.cc:2547–2556`) after repeated 1 s idle cycles with no visible targets.

#### Observable impact

- Monsters lose targets ~1.75 s after the player leaves view (via `creature_on_think` visibility check), instead of re-evaluating organically on the next `IdleStimulus` cycle.
- Monsters go to `Sleeping` state prematurely — after a single failed idle stimulus + 1.75 s think sweep, instead of after repeated 1 s idle cycles with no visible targets.
- The transition from "chasing" to "idle" to "sleeping" feels abrupt and mechanical, not organic.
- The `monster_combat_reschedule_if_stalled` call (`monster_ai.rs:786–788`) is a stall-rescue band-aid that masks the root cause — it re-arms idle when the ToDoQueue has stalled, but the stall itself is caused by RC2 (missing trailing wait).

#### Why this was likely introduced

The 1098 `check_creatures` (`creature_think.rs:57–105`) does call `monster_on_think` on a 1 Hz bucket sweep — that is correct for 1098 (TFS `Game::checkCreatures` → `Creature::onThink`). The 772 path was likely copied from the 1098 path without recognizing that 772 `ProcessCreatures` is a different function with a different contract (regen only, no AI).

#### Fix (implemented 2026-06-28)

Replaced `process_creatures_772` with C++ `ProcessCreatures` logic: death safety net only (`HP <= 0 && !IsDead → Death()`). Regen is already handled by `process_skills_772` → `process_player_fed_regen_772` (`process_skills.rs:29`). Logout is handled by `process_connections_772` / `pending_idle_kick_772`. No AI think, no target validation, no idle status update.

**Tests added:** `process_creatures_772_does_not_call_on_think`, `process_creatures_772_applies_death_safety`, `process_creatures_772_does_not_clear_targets` (`creature_think.rs`). All 457 tests pass.

<ref_file file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/creature_think.rs" />

---

### 4.2 Root Cause #2 — Missing unconditional trailing `ToDoWait(1000)` when idle produces no action

**Severity:** Critical  
**Status:** **Fixed** (2026-06-28)  
**Symptom contribution:** "Occasional pauses" (~750 ms extra delay before re-evaluation)

#### C++ 772 `IdleStimulus` (`crnonpl.cc:2938–2939`)

```cpp
// ALWAYS at the end of IdleStimulus, regardless of what happened above:
this->ToDoWait(1000);
this->ToDoStart();
```

This guarantees the monster re-thinks in **1000 ms** even if no action was queued (no target, roam failed, spell on cooldown). The monster never stalls — it always has a pending wakeup.

#### Rust port `monster_idle_stimulus_inner` (`idle_stimulus.rs:1055–1183`)

The Rust port does **not** have an unconditional trailing wait. Instead it relies on:

1. `monster_idle_reschedule_target_bound_if_parked` (`idle_stimulus.rs:1186–1238`) — only fires when there's a **chase target** (`follow_target.or(attack_target).is_some()`).
2. `monster_idle_maybe_enqueue_at_goal_wait` (`idle_stimulus.rs:1688–1731`) — only fires for `DistDance`/`MeleeDance` branches.

When a monster has **no target** and **roam fails** (no valid adjacent tile), the outcome is `Hold` (`idle_stimulus.rs:2035`):

- `monster_idle_prepare_and_enqueue_go` does nothing for `Hold` on the beat-driven path (`idle_stimulus.rs:2097–2102`).
- `monster_idle_maybe_enqueue_attack` returns `false` (no target).
- `monster_idle_maybe_enqueue_at_goal_wait` returns early (branch is `Roam`, not `DistDance`/`MeleeDance`).
- `monster_idle_reschedule_target_bound_if_parked` returns early (no chase target).

**Result: no wakeup is scheduled. The monster stalls until the next `process_creatures_772` (~1.75 s), then gets sent to sleep by RC1.**

#### Observable impact

- Idle monsters that can't roam (cornered, surrounded) pause for ~1.75 s instead of 1 s.
- Monsters that just lost their target and can't immediately find a new one pause for ~1.75 s before going to sleep, instead of re-evaluating every 1 s.
- The 750 ms extra delay is perceptible as a "hiccup" in monster behavior.
- Combined with RC1, the stall + premature sleep creates a visible "freeze then disappear from activity" pattern.

#### Why this matters

The C++ `IdleStimulus` is structured as a single function with a single exit point — the trailing `ToDoWait(1000) + ToDoStart` is unreachable to skip. The Rust port decomposed `IdleStimulus` into many small helper functions (`monster_idle_prepare_and_enqueue_go`, `monster_idle_maybe_enqueue_attack`, etc.), each with their own early returns. The decomposition is good for readability, but it lost the unconditional tail that guarantees a wakeup.

#### Fix (implemented 2026-06-28)

Added a trailing fallback at the end of `monster_idle_stimulus_inner` that mirrors the C++ idle-wandering catch-all (`crnonpl.cc:2938–2939`). When no wakeup was armed by the branches above (`todo.is_empty() && next_wakeup.is_none()`), it enqueues `ToDoWait(1000) + ToDoStart()` via `idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS)`. The `already_armed` guard prevents double-scheduling when an earlier branch already armed a wakeup (e.g. chase `QueuedGo` with `wait_after`, or `monster_idle_reschedule_target_bound_if_parked`).

**Tests added:** `rc2_idle_stimulus_always_schedules_wakeup_when_no_action_queued`, `rc2_idle_stimulus_does_not_double_schedule_when_already_armed`, `rc2_idle_trailing_wait_is_1000ms` (`idle_stimulus.rs`). All 457 tests pass.

<ref_file file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/idle_stimulus.rs" />

---

### 4.3 Bug #3 — Inverted `is_summon` check in `monster_sleep_wake_on_creature_move`

**Severity:** High  
**Symptom contribution:** Erratic sleep/wake transitions

#### C++ 772 `CreatureMoveStimulus` (`crnonpl.cc:2943–2982`)

```cpp
void TMonster::CreatureMoveStimulus(uint32 CreatureID, int Type){
    if(this->State == SLEEPING && Type != OBJECT_DELETED){
        TCreature *Creature = GetCreature(CreatureID);
        if(Creature == NULL) return;
        if(Creature->Type == NPC) return;
        if(Creature->Type == MONSTER && !((TMonster*)Creature)->IsPlayerControlled()){
            return;  // Don't wake for wild monsters
        }
        this->State = IDLE;
        this->ToDoYield();  // Wake immediately
    }
}
```

Wake rules: **players** → wake, **player summons** (`IsPlayerControlled()`) → wake, **wild monsters** → don't wake, **NPCs** → don't wake.

#### Rust port `monster_sleep_wake_on_creature_move` (`idle_stimulus.rs:314–320`)

```rust
let should_wake = self.creatures.get(moved_id).is_some_and(|k| match k {
    CreatureKind::Npc(_) => false,
    CreatureKind::Monster(m) => {
        !m.base.is_summon() && m.opponent_ids.is_empty() && !m.is_hostile  // ← INVERTED
    }
    CreatureKind::Player(_) => true,
});
```

This returns `true` (wake) for **wild monsters** (`!is_summon()`) and `false` (don't wake) for **player summons** (`is_summon()`). This is the opposite of C++.

The `opponent_ids.is_empty() && !m.is_hostile` guards were likely added to prevent wild monsters from waking each other into infinite aggro chains — a reasonable concern, but the `is_summon()` polarity is inverted, so the guard protects the wrong population.

#### Observable impact

- Sleeping monsters wake erratically when wild monsters roam nearby (should stay asleep).
- Sleeping monsters don't wake when a player's summon walks past (should wake).
- Contributes to erratic sleep/wake transitions — monsters seem to "twitch" awake for no reason.

<ref_file file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/idle_stimulus.rs" />

---

### 4.4 Bug #4 — `monster_update_idle_status` sends monsters to sleep from the think sweep

**Severity:** Medium (resolved by RC1 fix)  
**Symptom contribution:** Premature sleep

#### C++ 772

Sleep transition happens **only** inside `IdleStimulus` (`crnonpl.cc:2547–2556`), when no targets are visible AND no master. The monster must go through repeated 1 s idle cycles with no visible targets before sleeping.

#### Rust port

`monster_update_idle_status` (`monster_targets.rs:353–358`) is called from `monster_native_on_think` (`monster_ai.rs:766`), which runs in `process_creatures_772`. It sets `state = Sleeping` when `opponent_ids.is_empty()` — regardless of whether there are visible creatures on the same floor.

Combined with RC1 (which calls `monster_on_think` from `process_creatures_772`), this sends monsters to sleep after a single failed idle + 1.75 s, instead of after repeated 1 s idle cycles.

#### Resolution

**Resolved by RC1 fix (2026-06-28).** With `monster_on_think` removed from `process_creatures_772`, `monster_update_idle_status` is no longer called from the 772 think sweep. It is still called from `monster_on_creature_move` (`monster_events.rs:164`), which is correct (re-evaluate on creature movement). The `should_sleep` conditions in `monster_idle_772_acquire_target` (`idle_stimulus.rs:635–646`) now run only inside `IdleStimulus`, matching C++ `crnonpl.cc:2547–2556`.

<ref_file file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/monster_targets.rs" />

---

### 4.5 Minor #5 — `creature_can_see` Z-level split stricter than 772 `IsVisible`

**Severity:** Low  
**Symptom contribution:** Underground monsters won't aggro on surface players (edge case)

#### C++ 772 `IsVisible` (`connections.cc:357–378`)

```cpp
if(PlayerZ <= 7){
    if(z > 7) return false;
}else{
    if(std::abs(PlayerZ - z) > 2) return false;  // underground CAN see surface (|8-7|=1 ≤ 2)
}
```

#### Rust `creature_can_see` (`game_world_spectators.rs:59–70`)

```rust
if my_z <= 7 {
    if tz > 7 { return false; }
} else if my_z >= 8 {
    if tz < 8 { return false; }  // ← C++ doesn't have this: blocks underground from seeing surface
    if (my_z - tz).abs() > 2 { return false; }
}
```

The Rust port blocks underground monsters (z ≥ 8) from seeing surface creatures (z ≤ 7) in AI target selection. C++ 772 allows it (`|8-7|=1 ≤ 2`). This is the TFS `canSee` logic (`creature.cpp`), not the 772 `IsVisible` logic. The Rust port uses the TFS version for AI, which is slightly stricter on Z than 772.

#### Observable impact

- Underground monsters (z ≥ 8) won't aggro on surface players (z ≤ 7) even when adjacent on a ramp.
- Minor; affects only multi-floor ramp scenarios.

<ref_file file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/game_world_spectators.rs" />

---

## 5. Fix Plan

### 5.1 Priority order

| Priority | Finding | Effort | Impact | Status |
|----------|---------|--------|--------|--------|
| P0 | RC1 — Remove AI think from `process_creatures_772` | Medium | Eliminates abrupt transitions | **Done** (2026-06-28) |
| P0 | RC2 — Add unconditional trailing `ToDoWait(1000)` to idle | Small | Eliminates pauses | **Done** (2026-06-28) |
| P1 | B3 — Fix inverted `is_summon` check | Trivial | Corrects sleep/wake | Open |
| P2 | B4 — Verify sleep transition after RC1 fix | Small | Defense in depth | **Done** (resolved by RC1) |
| P3 | M5 — Align Z-level split with 772 `IsVisible` | Small | Edge case correctness | Open |

### 5.2 Fix RC1 — `process_creatures_772` regen-only — **IMPLEMENTED**

**File:** `crates/tfs-rust-core/src/creature_think.rs`

**Change:** Replace the `monster_on_think` / `npc_on_think` calls in `process_creatures_772` with the C++ `ProcessCreatures` regen logic. AI think is already driven by the ToDoQueue / `IdleStimulus` / `CreatureMoveStimulus` / `DamageStimulus` — no 1 Hz think sweep is needed on the 772 path.

**Target shape:**

```rust
/// 772 `ProcessCreatures` — regen + death safety only (`crmain.cc:1075–1138`).
/// AI think is driven by the ToDoQueue / IdleStimulus, NOT this sweep.
pub fn process_creatures_772(&mut self) {
    let ids: Vec<CreatureId> = self.creatures.iter()
        .map(|(id, _)| id)
        .collect();

    for cid in ids {
        // C++ `ProcessCreatures` regen: `RoundNr % SKILL_FED == 0` → +1 HP, +4 mana.
        // Already handled by `process_player_fed_regen_772` in `process_skills_772`
        // (`process_skills.rs:29`). Confirm no double-regen after this change.

        // C++ death safety: HP <= 0 but not dead → Death().
        if self.creature_alive_for_think(cid) {
            let hp = self.creatures.get(cid).map(|k| k.base().health).unwrap_or(0);
            if hp <= 0 {
                self.apply_creature_death(cid);
            }
        }

        // C++ logout processing is handled by `process_connections_772` / `pending_idle_kick_772`.
        // No AI think, no target validation, no idle status update here.
    }
}
```

**Notes:**
- The `process_player_fed_regen_772` call already exists in `process_skills_772` (`process_skills.rs:29`). Verify it is not duplicated after this change.
- The `monster_combat_reschedule_if_stalled` stall-rescue in `monster_native_on_think` (`monster_ai.rs:786–788`) becomes unreachable on the 772 path — that is correct, because RC2 eliminates the stall it was rescuing.
- `check_creatures` (the 1098 path) is unchanged — it correctly calls `monster_on_think` on a 1 Hz bucket sweep.
- NPC think (`npc_on_think`) on the 772 path needs separate consideration — C++ `ProcessCreatures` does not call NPC think either; NPC AI on 772 is also ToDoQueue-driven. Confirm against `crnonpl.cc` NPC paths before removing.

**Tests to add:**
- `process_creatures_772_does_not_call_monster_on_think` — assert no `on_think` event fires after `process_creatures_772` with a monster in the world.
- `process_creatures_772_applies_death_safety` — assert a monster with HP ≤ 0 is killed.
- `process_creatures_772_does_not_clear_targets` — assert follow/attack targets survive a `process_creatures_772` call when the target is out of view but within 10 tiles.

### 5.3 Fix RC2 — Unconditional trailing `ToDoWait(1000)` — **IMPLEMENTED**

**File:** `crates/tfs-rust-core/src/idle_stimulus.rs`

**Change:** Add an unconditional trailing `ToDoWait(1000) + ToDoStart` at the end of `monster_idle_stimulus_inner`, matching C++ `crnonpl.cc:2938–2939`. Only skip it if the idle already armed a wakeup (todo non-empty or `next_wakeup` is set).

**Target shape** (append to `monster_idle_stimulus_inner`, after line 1182):

```rust
// C++ `IdleStimulus` always ends with `ToDoWait(1000); ToDoStart();` (`crnonpl.cc:2938–2939`).
// This guarantees a re-think in 1000 ms even when no action was queued (no target, roam
// failed, spell on cooldown). Without it, a monster with no target and no roam step stalls
// until the next `process_creatures_772` (~1.75 s) — the source of "occasional pauses."
// Skip only when the idle above already armed a wakeup (todo non-empty or next_wakeup set).
let already_armed = self.creatures.get(cid).is_some_and(|k| {
    let base = k.base();
    !base.todo.is_empty() || base.next_wakeup.is_some()
});
if !already_armed {
    self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
}
```

**Notes:**
- `monster_idle_reschedule_target_bound_if_parked` (`idle_stimulus.rs:1186`) already covers the "parked with chase target" case — the new tail covers the "no chase target" case.
- `monster_idle_maybe_enqueue_at_goal_wait` (`idle_stimulus.rs:1688`) covers `DistDance`/`MeleeDance` — the new tail covers `Roam`/`Hold`/`Noway`.
- The `already_armed` guard prevents double-scheduling when an earlier branch already armed a wakeup (e.g. `QueuedGo` with `wait_after`).

**Tests to add:**
- `idle_stimulus_always_schedules_wakeup_when_no_action_queued` — assert `next_wakeup.is_some()` after `monster_idle_stimulus` on a monster with no target and no roam step.
- `idle_stimulus_does_not_double_schedule_when_already_armed` — assert `next_wakeup` is unchanged when idle already armed a wakeup.
- `idle_stimulus_roam_hold_schedules_1000ms_wakeup` — assert `next_wakeup == server_ms + 1000` after a `Hold` outcome.

### 5.4 Fix B3 — Inverted `is_summon` check

**File:** `crates/tfs-rust-core/src/idle_stimulus.rs`

**Change:** Fix the `is_summon` polarity in `monster_sleep_wake_on_creature_move`.

**Diff:**

```rust
// Before (idle_stimulus.rs:314–320):
CreatureKind::Monster(m) => {
    !m.base.is_summon() && m.opponent_ids.is_empty() && !m.is_hostile
}

// After:
// C++ `CreatureMoveStimulus` wakes for players and player-controlled summons
// (`IsPlayerControlled()`), NOT wild monsters (`crnonpl.cc:2955–2958`).
// `is_summon()` in Rust maps to `IsPlayerControlled()` (player-owned summon).
CreatureKind::Monster(m) => {
    m.base.is_summon()
}
```

**Notes:**
- The `opponent_ids.is_empty() && !m.is_hostile` guards are removed — C++ does not have them. They were a band-aid for the inverted polarity. If wild-monster-on-wild-monster aggro chains become a problem, re-add a guard that specifically targets wild monsters (`!is_summon()`) rather than inverting the summon check.
- `is_summon()` in Rust (`creature/base.rs`) returns `true` when `master.is_some()` — this maps to C++ `IsPlayerControlled()` for player-owned summons. Verify this mapping is correct for NPC-owned summons (C++ `IsPlayerControlled()` returns false for NPC summons).

**Tests to add:**
- `sleep_wake_wakes_for_player_summon` — assert a sleeping monster wakes when a player summon moves nearby.
- `sleep_wake_does_not_wake_for_wild_monster` — assert a sleeping monster stays asleep when a wild monster moves nearby.
- `sleep_wake_wakes_for_player` — assert a sleeping monster wakes when a player moves nearby (unchanged).

### 5.5 Fix B4 — Verify sleep transition after RC1 — **RESOLVED BY RC1**

**File:** `crates/tfs-rust-core/src/monster_targets.rs`

**Change:** After RC1, `monster_update_idle_status` is no longer called from the 772 think sweep. It is still called from `monster_on_creature_move` (`monster_events.rs:164`), which is correct (re-evaluate on creature movement). Verify that the sleep transition conditions match C++:

- C++ sleeps when: no targets visible on same floor AND no master (`crnonpl.cc:2547–2556`).
- Rust currently sleeps when: `opponent_ids.is_empty()` (`monster_targets.rs:355`).

If `opponent_ids` is populated by `monster_on_creature_found` (visible creature enter range), then `opponent_ids.is_empty()` is a reasonable proxy for "no targets visible." But it does not check same-floor — a monster on a different floor with a remembered opponent won't sleep. Confirm this matches C++ behavior (C++ `IdleStimulus` checks `Target == 0` after the target selection loop, which already filters by same-floor).

**No code change likely needed** — this is a verification task after RC1 lands.

### 5.6 Fix M5 — Align Z-level split with 772 `IsVisible`

**File:** `crates/tfs-rust-core/src/game_world_spectators.rs`

**Change:** Remove the `tz < 8` block for underground viewers in `creature_can_see`, matching C++ 772 `IsVisible` (`connections.cc:357–378`).

**Diff:**

```rust
// Before (game_world_spectators.rs:59–70):
if my_z <= 7 {
    if tz > 7 { return false; }
} else if my_z >= 8 {
    if tz < 8 { return false; }  // ← remove this line
    if (my_z - tz).abs() > 2 { return false; }
}

// After:
if my_z <= 7 {
    if tz > 7 { return false; }
} else if my_z >= 8 {
    if (my_z - tz).abs() > 2 { return false; }
}
```

**Notes:**
- This changes AI target selection visibility, not client viewport visibility (`can_see_position` uses `protocol_can_see`, which is separate). Confirm `protocol_can_see` is not affected.
- The TFS `canSee` logic (`creature.cpp`) does have the `tz < 8` block — this is a 1098 vs 772 divergence. Gate the change on `beat_driven_loop` if both eras share `creature_can_see`, or split into two functions.
- Low priority — only affects multi-floor ramp scenarios where an underground monster is adjacent to a surface player.

**Tests to add:**
- `creature_can_see_underground_to_surface` — assert an underground monster (z=8) can see a surface player (z=7) within range.

---

## 6. Verification Plan

### 6.1 Per-fix verification

| Fix | `cargo check` | `cargo clippy` | `cargo test` | Manual | Status |
|-----|---------------|----------------|--------------|--------|--------|
| RC1 | ✅ | ✅ (no new warnings) | 3 new tests pass | Observe monster chase→idle→sleep transition timing | **Done** |
| RC2 | ✅ | ✅ (no new warnings) | 3 new tests pass | Observe no pauses when monster is cornered | **Done** |
| B3 | — | — | — | Observe sleep/wake with player summon nearby | Pending |
| B4 | ✅ | ✅ | Existing tests pass | Confirm no regression in sleep timing | **Done** (resolved by RC1) |
| M5 | — | — | — | Multi-floor ramp scenario | Pending |

**Full suite: 457 passed, 0 failed, 2 ignored** (2026-06-28).

### 6.2 Integration verification

RC1 + RC2 integration verified against the existing sim harness and chase tests (all 457 tests pass). Remaining B3/M5 verification pending.

```bash
rtk cargo test --package tfs-rust-core -- sim_harness
rtk cargo test --package tfs-rust-core -- chase
rtk cargo test --package tfs-rust-core -- idle
```

Key scenarios to observe:
- `kite_cyclops_one_real` — chase cadence should be unchanged (RC1/RC2 do not affect active chase). ✅
- Idle monster in a corner — should re-evaluate every 1 s (RC2), not stall for 1.75 s. ✅
- Player walks past sleeping monster — monster wakes immediately (B3 fix for player path, already correct). Pending B3.
- Player summon walks past sleeping monster — monster wakes immediately (B3 fix). Pending B3.
- Wild monster walks past sleeping monster — monster stays asleep (B3 fix). Pending B3.

### 6.3 Lessons captured

Added to `tasks/lessons.md` (entries #85, #86, 2026-06-28):

- **#85 — 772 `ProcessCreatures` is regen-only** — do not call AI think from the 1 Hz creature sweep. AI is ToDoQueue-driven.
- **#86 — 772 `IdleStimulus` idle-wandering catch-all** — decomposed helpers lost the unconditional `ToDoWait(1000) + ToDoStart` tail. Re-added with `already_armed` guard.
- **772 `CreatureMoveStimulus` wake polarity** — pending B3 fix.

---

## 7. References

### 7.1 C++ source (772 decompile)

- `cract.cc:783–898` — `TCreature::Execute` (ToDoList drain loop)
- `cract.cc:901–951` — `CalculateDelay` (per-action delay)
- `cract.cc:1010–1024` — `ToDoStart` (schedule + +1 clamp)
- `cract.cc:1026–1031` — `ToDoYield` (zero-delay wake)
- `cract.cc:1530–1538` — `NotifyGo` (walk quantization to beat)
- `crmain.cc:1075–1138` — `ProcessCreatures` (regen-only)
- `crmain.cc:1142–1158` — `MoveCreatures` (ToDoQueue drain)
- `crmain.cc:888–961` — `CreatureMoveStimulus` (close-chase re-arm)
- `crnonpl.cc:2278–2343` — `DamageStimulus` (wake on damage)
- `crnonpl.cc:2345–2941` — `IdleStimulus` (full monster think)
- `crnonpl.cc:2418–2435` — target validation (lose conditions)
- `crnonpl.cc:2470–2544` — target selection (strategy)
- `crnonpl.cc:2547–2556` — sleep transition
- `crnonpl.cc:2900–2940` — idle wandering (10 random directions)
- `crnonpl.cc:2938–2939` — unconditional trailing `ToDoWait(1000)`
- `crnonpl.cc:2943–2982` — `CreatureMoveStimulus` (sleep wake)
- `main.cc:320–340` — `AdvanceGame` subsystem counters
- `main.cc:444–453` — lag guard (1000 ms)
- `main.cc:493–496` — beat coalescing (`SigAlarmCounter`)
- `sending.cc:658–694` — `SendMoveCreature` (0x6D)
- `sending.cc:615–635` — `SendChangeField` (0x6B)
- `sending.cc:597–656` — `SendAddField` / `SendDeleteField` (0x6A / 0x6C)
- `connections.cc:357–378` — `IsVisible` (client viewport, Z rules)
- `containers.hh:150–227` — `priority_queue` (verbatim heap)

### 7.2 Rust files

- `crates/tfs-rust-core/src/creature_think.rs` — `process_creatures_772` (RC1)
- `crates/tfs-rust-core/src/idle_stimulus.rs` — `monster_idle_stimulus_inner` (RC2), `monster_sleep_wake_on_creature_move` (B3)
- `crates/tfs-rust-core/src/monster_targets.rs` — `monster_update_idle_status` (B4)
- `crates/tfs-rust-core/src/game_world_spectators.rs` — `creature_can_see` (M5)
- `crates/tfs-rust-core/src/todo_queue.rs` — verbatim heap port (clean)
- `crates/tfs-rust-core/src/walk/mod.rs` — `drain_todo_queue` (clean)
- `crates/tfs-rust-core/src/walk/walk_timing.rs` — beat quantization (clean)
- `crates/tfs-rust-core/src/game_loop.rs` — `run_game_loop_772` (clean)
- `crates/tfs-rust-core/src/game_world_tick.rs` — `advance_beat_772` (clean)
- `crates/tfs-rust-core/src/subsystem_counters_772.rs` — staggered counters (clean)
- `crates/tfs-rust-core/src/creature_todo.rs` — `CreatureAction`, `todo_start_from_action` (clean)
- `crates/tfs-rust-core/src/monster_events.rs` — `monster_on_creature_move` (clean)
