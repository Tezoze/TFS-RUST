# Monster AI Beat-Driven Loop Audit

Audit of the monster AI path through the beat-driven loop, comparing the Rust
implementation against the C++ 772 decompile.

**Date:** July 2026
**Scope:** `process_creature_todo` → `run_monster_todo_execute` →
`execute_creature_todo_action` → `finish_creature_todo_execute` →
`idle_stimulus` → `monster_idle_stimulus_inner`, plus the stimulus fan-out
(`monster_on_creature_move` / `monster_combat_creature_move_stimulus`),
`DamageStimulus`, `CreatureMoveStimulus`, and the `IdleStimulus` sub-blocks
(summon lifecycle, target acquisition, casting, walking, roam).

**C++ references:**
- `cract.cc:783-898` — `Execute`
- `cract.cc:901-951` — `CalculateDelay`
- `cract.cc:968-1024` — `ToDoStart` / `ToDoYield` / `ToDoClear`
- `crnonpl.cc:2304-2342` — `TMonster::DamageStimulus`
- `crnonpl.cc:2345-2939` — `TMonster::IdleStimulus`
- `crmain.cc:920-961` — `TCreature::CreatureMoveStimulus`
- `operate.cc:930-987` — `NotifyAllCreatures`
- `magic.cc:416-453` — `VictimShapeSpell`
- `crcombat.cc:520-529` — `TCombat::DelayAttack`

---

## Findings

### M1 — `DamageStimulus` over-yields on non-sleeping state changes (MEDIUM)

**C++** (`crnonpl.cc:2304-2342`): `ToDoYield()` is called **only** when
`State == SLEEPING` (the first branch). The non-sleeping branch
(IDLE→UNDERATTACK, →PANIC) changes state but does NOT yield — the next
scheduled wakeup handles the new state.

**Rust** (`idle_stimulus.rs:572-577`): Calls `creature_todo_yield` when
`state_changed || was_sleeping`. This yields on **every** damage hit that
changes state, not just SLEEPING→wake.

**Impact:** Extra `ToDoWait(0) + ToDoStart` on every damage hit while awake.
This preempts the current todo queue (prepending `Wait{0}`) and arms a new
wakeup, causing:
- Queue churn on every hit (an in-flight chase sequence gets yielded)
- Extra wakeups (performance)
- The `Wait{0}` lands on next beat (+1), delaying the in-flight action by one
  beat

---

### M2 — `DamageStimulus` adds 4000ms attack delay not in C++ (MEDIUM)

**C++** (`crnonpl.cc:2304-2342`): `DamageStimulus` only changes state and
yields. It does NOT set `EarliestAttackTime`. `EarliestAttackTime` is set by
`TCombat::DelayAttack(ms)` called from `Attack()` (`cract.cc:489` → 2000ms),
`CanToDoAttack` (`crcombat.cc:146,334,641` → 2000ms), and
`CreatureMoveStimulus` catch (`crcombat.cc:608` → 200ms).

**Rust** (`idle_stimulus.rs:573-576`): Calls
`m.base.delay_attack_ms(self.server_ms, 4000)` on
`state_changed || was_sleeping`. This 4000ms delay is not from any C++ source.

**Impact:** Monsters hit while awake wait 4 seconds before their next attack.
In C++, a monster hit while IDLE→UNDERATTACK can attack on the next idle drain
(subject to normal `EarliestAttackTime` from the last attack). In Rust, the
4000ms gate prevents retaliation for 4 full seconds. This makes monsters feel
sluggish when retaliating.

---

### M3 — Casting block: non-aggressive spells skipped when no target (MEDIUM)

**C++** (`crnonpl.cc:2682`): The casting gate is
`!Impact->isAggressive() || (this->Target != 0 && this->Target != this->Master)`.
Non-aggressive spells (healing, speed, outfit) have `!isAggressive() == true`
→ always cast, even with `Target == 0`.

**Rust** (`idle_stimulus.rs:1047-1052`): Early-returns when `cast_target` is
None, skipping ALL spells including non-aggressive self-healing/self-buff.

**Impact:** A monster with no target (idle, roaming) never heals itself or
casts self-buffs in Rust. In C++, a rat at low HP would heal itself while
roaming; in Rust it stays at low HP until it acquires a target.

---

### M4 — Casting block: missing protection zone check for spell victims (MEDIUM)

**C++** (`magic.cc:436-438`): `VictimShapeSpell` checks
`Impact->isAggressive() && IsProtectionZone(Victim->posx, ...)` → returns
(skips the spell). Same for `DestinationShapeSpell`.

**Rust** (`idle_stimulus.rs:1094-1104`): The `Victim`/`Destination` arm checks
`monster_sight_clear` but does NOT check if the target is in a protection
zone. `monster_idle_apply_spell_impact` has no PZ check either.

**Impact:** Monsters cast aggressive spells (damage, conditions) at targets
standing in protection zones. In C++, these spells are blocked.

---

### L5 — Casting block: aggressive spells not gated against `Target == Master` (LOW)

**C++** (`crnonpl.cc:2682`): Aggressive spells require
`Target != Master` — a summon cannot cast aggressive spells at its master.

**Rust** (`idle_stimulus.rs:1064-1125`): No `target == master` check in the
casting loop. A summon whose `attack_target` happens to be its master would
cast aggressive spells at it.

**Impact:** Edge case — summons normally have their target set to the master's
attack target, not the master itself. Only observable if the target is
manually set to the master.

---

### L6 — Casting block: `monster_idle_suppress_adjacent_melee_spell` is a Rust addition (LOW)

**Rust** (`idle_stimulus.rs:1087-1089, 1141-1153`): Suppresses spells when
`dist <= 1` and the monster is a melee fighter (`melee_skill > 0`,
`target_distance <= 1`).

**C++** (`crnonpl.cc:2571-2740`): No such suppression. The only casting skip
is `IsFleeing() && random(1,3) != 1`.

**Impact:** A melee monster adjacent to its target skips spell casting in Rust
but casts spells in C++. This changes the RNG stream (the
`rand() % Delay` draw is skipped) and the combat behavior (e.g., a dragon
adjacent to a player would cast fire attacks in C++ but not in Rust).

---

### L7 — Missing `LifeEndRound` despawn (LOW)

**C++** (`crnonpl.cc:2352-2357`):
`if(this->LifeEndRound != 0 && this->LifeEndRound <= RoundNr)` →
`StartLogout(true, true)` + `State = SLEEPING` + return. Monsters with a
limited lifetime (e.g., summoned creatures with a duration) despawn when their
lifetime expires.

**Rust**: No `LifeEndRound` field or check. Timed summons live forever.

**Impact:** Summons with a configured lifetime never expire. This is a missing
feature, not a behavioral divergence in the beat loop itself.

---

### L8 — Missing home range despawn for non-summons (LOW)

**C++** (`crnonpl.cc:2407-2416`):
`if(!MonsterhomeInRange(this->Home, this->posx, this->posy, this->posz))` →
`StartLogout(true, true)` + `State = SLEEPING` + return. Monsters outside
their home radius despawn.

**Rust**: `home_radius` is used for roam step validation
(`monster_roam_leash_radius`) but there's no despawn check in
`monster_idle_stimulus_inner`. A monster pushed/kicked outside its home range
by another creature would keep operating instead of despawning.

**Impact:** Edge case — the roam leash prevents voluntary roaming outside the
radius. Only observable when external forces (push, kick, teleport) move the
monster outside.

---

### L9 — Move stimulus radius: 11 vs 10 (LOW)

**C++** (`operate.cc:937`): `StimulusRadius = 10` — `NotifyAllCreatures`
searches a 10-tile radius.

**Rust** (`monster_events.rs:71, monster_ai.rs:45`):
`collect_spatial_spectators` uses `MAP_MAX_VIEWPORT = 11`.

**Impact:** Monsters 11 tiles away receive `CreatureMoveStimulus` in Rust but
not in C++. This could cause extra idle repaths for monsters at the edge of
the stimulus range. Already documented as a known parity gap (GL#24).

---

## Verified correct (no divergence)

| Mechanism | C++ reference | Rust status |
|-----------|--------------|-------------|
| `Execute` `while(true)` zero-delay chaining | `cract.cc:784` | Fixed (audit #14) |
| `CalculateDelay(TDWait)` `EarliestWalkTime` floor | `cract.cc:906-910` | Fixed (audit #14) |
| `CalculateDelay(TDGo)` — `EarliestWalkTime` gate | `cract.cc:918-923` | `todo_start_go_delay` |
| `CalculateDelay(TDAttack)` — `max(EarliestAttack, EarliestSpell)` | `cract.cc:934-943` | `todo_attack_delay_ms` |
| `CalculateDelay(TDUse)` — `EarliestMultiuseTime` gate | `cract.cc:925-932` | `multiuse_gate_delay_ms` |
| `NotifyGo` — `ceil(Delay/Beat)*Beat` | `cract.cc:1532-1534` | `ceil_to_walk_quantizer` |
| `Execute` catch — `ToDoClear` + `ToDoYield`/`ToDoWait(1000)` | `cract.cc:870-888` | `apply_todo_result_catch` |
| `Execute` `Stop` check | `cract.cc:891-897, 797-801` | `finish_creature_todo_execute` |
| `IdleStimulus` `LockToDo` gate | `crnonpl.cc:2346` | `idle_stimulus` checks `todo.locked` |
| `IdleStimulus` summon lifecycle | `crnonpl.cc:2359-2405` | `monster_idle_summon_lifecycle` |
| `IdleStimulus` lose target (range/PZ/house/invisible/LoseTarget) | `crnonpl.cc:2418-2435` | `monster_idle_lose_existing_target` |
| `IdleStimulus` state reset (not PANIC/UNDERATTACK) | `crnonpl.cc:2437-2439` | `monster_idle_reset_combat_state` |
| `IdleStimulus` talk gate (`rand()%50`) | `crnonpl.cc:2442-2468` | `monster_idle_try_talk` |
| `IdleStimulus` target acquisition (`Strategy[]`) | `crnonpl.cc:2470-2566` | `monster_idle_acquire_target` |
| `IdleStimulus` ShouldSleep | `crnonpl.cc:2547-2557` | summon case dead (`Target` always set to `Master`) |
| `IdleStimulus` casting flee gate | `crnonpl.cc:2581` | `parity_random(1, 3) != 1` |
| `IdleStimulus` walking — flee/master/combat/dance/roam | `crnonpl.cc:2743-2939` | `monster_idle_classify_walk_branch` + `monster_idle_execute_walk_branch` |
| `IdleStimulus` catch-all `ToDoWait(1000)` | `crnonpl.cc:2920-2939` | RC2 tail (`idle_enqueue_wait_and_start`) |
| `CreatureMoveStimulus` close-chase re-arm | `crmain.cc:920-961` | `monster_combat_creature_move_stimulus` |
| `CreatureMoveStimulus` head-is-attack gate | `crmain.cc:931-932` | AI#26 fix (`front() == Attack`) |
| `CreatureMoveStimulus` 200ms strike gate | `crmain.cc:925` | `earliest_attack_ms <= server_ms + 200` |
| `CreatureMoveStimulus` `ToDoClear` + `ToDoWait(200)` + `ToDoAttack` | `crmain.cc:952-957` | `monster_combat_creature_move_stimulus` |
| `CreatureMoveStimulus` NOWAY catch → clear+roam | `crnonpl.cc:2890-2898` | `monster_idle_noway_clear_and_roam` |
| `DamageStimulus` state transitions | `crnonpl.cc:2304-2342` | correct (but see M1/M2 for yield/delay divergence) |
| `DamageStimulus` SLEEPING→PANIC/UNDERATTACK | `crnonpl.cc:2308-2314` | correct |
| `ToDoStart` `+1` clamp | `cract.cc:1016` | `todo_start_from_action` |
| `ToDoYield` `ToDoWait(0)` + `ToDoStart` | `cract.cc:1001` | `creature_todo_yield` |
| `ToDoClear` sets `LockToDo = false` | `cract.cc:984` | `player_todo_clear` / `monster_exhausted_wait` |
| `ToDoAdd` preempt (`LockToDo` → clear+snapback) | `cract.cc:992-996` | `player_todo_clear_with_snapback` |

---

## Priority ranking

| ID | Severity | Title |
|----|----------|-------|
| M1 | Medium | `DamageStimulus` over-yields on non-sleeping state changes |
| M2 | Medium | `DamageStimulus` adds 4000ms attack delay not in C++ |
| M3 | Medium | Casting: non-aggressive spells (healing/buff) skipped when no target |
| M4 | Medium | Casting: missing protection zone check for spell victims |
| L5 | Low | Casting: aggressive spells not gated against `Target == Master` |
| L6 | Low | Casting: `suppress_adjacent_melee_spell` is a Rust addition |
| L7 | Low | Missing `LifeEndRound` despawn (timed summons) |
| L8 | Low | Missing home range despawn for non-summons |
| L9 | Low | Move stimulus radius 11 vs 10 (pre-existing, GL#24) |
