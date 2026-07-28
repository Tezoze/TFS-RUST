# Monster AI + ToDo Scheduler — 772 Parity Audit

**Date:** 2026-07-27
**Scope:** `TMonster::IdleStimulus` / `DamageStimulus` / `CreatureMoveStimulus` / `MovePossible` and the
global ToDo action scheduler (`TCreature::Execute`, `CalculateDelay`, `ToDoStart/Wait/Clear/Yield`,
`MoveCreatures`, `priority_queue`).

**Reference (read directly for this audit):**
`reference/cipsoft-772/tibia-game-master/src/{crnonpl.cc,cract.cc,crmain.cc,cr.hh,enums.hh,containers.hh}`

**Rust under audit:**
`crates/tfs-rust-core/src/{idle_stimulus.rs,monster_ai.rs,monster_distance_step.rs,monster_events.rs,`
`monster_push.rs,creature_todo.rs,todo_queue.rs,walk/mod.rs,creature/monster.rs}`

**Verdict:** the scheduler core and the shape of `IdleStimulus` are a faithful, well-cited port. The real
gaps are concentrated in (a) **monster attribute data that is never populated**, (b) **two whole
IdleStimulus branches that have no Rust counterpart**, and (c) **added Rust-only behavior in
`DamageStimulus`** that has no decompile basis.

---

## 0. Tooling note (read this first)

`reference/cipsoft-772/tibia-game-master/` is matched by `.gitignore:190` (`tibia-game-master/`).
Ripgrep-based search tools honor `.gitignore`, so **any agent or search that relies on grep silently
reports the 772 decompile as "not present in the repo"** and then "audits" the Rust citations against
themselves. Two of three audit passes for this document failed exactly that way before the C++ was read
with direct file reads.

**Recommendation:** add `!reference/**` (negated) to `.gitignore`, or a `.ignore` file with
`!reference/`, so search tooling can see the reference stack.

---

## 1. Monster AI — BUGS

### AI-1 `Strategy[]` is never populated — every monster targets NEAREST
**Severity: BUG (data/behavior)**

C++ — `crnonpl.cc:2474-2483`, thresholds loaded in `crmain.cc:1475-1484`:
```cpp
int Strategy = 0;
int StrategyRoll = random(0, 99);
while(Strategy < (NARRAY(TRaceData::Strategy) - 1)){
    if(StrategyRoll < RaceData[this->Race].Strategy[Strategy]) break;
    StrategyRoll -= RaceData[this->Race].Strategy[Strategy];
    Strategy += 1;
}
```
```cpp
}else if(strcmp(Identifier, "strategy") == 0){
    Race->Strategy[0] = Script.readNumber(); ... Race->Strategy[3] = ...
```

Rust — the roll itself is correct (`idle_stimulus.rs:696-718`, `:957-961`), but the thresholds are
**hardcoded defaults that no loader ever overwrites**. `creature/monster.rs:103-105` and `:137-139`:
```rust
strategy_nearest: 100,
strategy_health: 0,
strategy_damage: 0,
```
`grep -rn "strategy_nearest"` over `crates/` returns only the two default sites, the struct fields, and
the copy at `monster.rs:307`. No XML/`.mon` parse path writes them.

**Measured blast radius** — `grep -h "^Strategy" reference/cipsoft-772/runtime/mon/*.mon | sort | uniq -c`:

| Strategy tuple | Races |
|---|---|
| `(100, 0, 0, 0)` — nearest only | 96 |
| **non-default** | **63** |

i.e. **63 of 159 races (40%) do not target nearest-only.** Real examples: `(70,0,30,0)` ×15,
`(80,10,10,0)` ×9, `(60,0,0,40)` ×7, `(10,10,20,60)`, and one race at `(0,0,0,100)` — *pure random
targeting*, which in Rust behaves as strictly-nearest.

**Impact:** `STRATEGY_LOWEST_HEALTH`, `STRATEGY_MOST_DAMAGE` and `STRATEGY_RANDOM` are dead code for
40% of the bestiary. Monsters that should switch to the weakest player, to whoever damaged them most,
or roll a random victim, all deterministically lock onto the Manhattan-nearest. Highly visible in group
PvE — the whole "monsters focus the weak one" texture of 772 is missing.

**Fix — the data already exists in-repo:** `reference/cipsoft-772/runtime/mon/*.mon` carries
`Strategy = (a, b, c, d)` verbatim for all 159 races. Import it into `MonsterAiConfig` (772 overlay
keyed by monster name, or a generated table); default `(100,0,0,0)` only when a race is absent.

---

### AI-2 `LoseTarget` is never populated — monsters never randomly drop aggro
**Severity: BUG (data/behavior)**

C++ — `crnonpl.cc:2431`:
```cpp
|| (this->Master == 0 && random(0, 99) < RaceData[this->Race].LoseTarget);
```
loaded via `crmain.cc:1473-1474` (`losetarget`).

Rust — check is present and correctly ordered (`idle_stimulus.rs:884-891`, master-gated, `random(0,99)`),
but `lose_target_percent` is `0` at `creature/monster.rs:102` / `:136` and never written from data.

**Measured blast radius** — `grep -h "^LoseTarget" reference/cipsoft-772/runtime/mon/*.mon`:

| `LoseTarget` | Races |
|---|---|
| `0` | 69 |
| **non-zero** | **90** |

Distribution of the non-zero set: `5` ×30, `50` ×24, `10` ×18, `3` ×7, `7` ×3, `20` ×2, and singletons
at `4, 6, 15, 25, 30, **95**`.

**Impact:** **90 of 159 races (57%)** are affected, and this is not a subtle tuning knob — a race with
`LoseTarget = 50` drops its target on **half of all idle passes**, and the `95` race abandons its target
almost immediately. In Rust every one of them chases relentlessly until a hard condition
(range/PZ/house/invisible) fires. This single missing field is probably the largest behavioral
difference in the whole monster AI: it is why 772 monsters wander off, lose interest, and let you walk
away, and why yours do not.

**Fix:** same import as AI-1 — `LoseTarget` is a plain integer in every `.mon` file.

---

### AI-3 Keep-distance band uses per-type `targetdistance`, not the hardcoded 4
**Severity: RISK (structural) — *not* currently observable. Downgraded from BUG after data check.**

**Measured:** `reference/cipsoft-772/runtime/mon/*.mon` has exactly **30** races with the
`DistanceFighting` flag; `data/monster/**` has exactly **29** files with `targetdistance="4"`, and the
sets are identical apart from the `human` race (gamemaster, not a spawnable monster). Every other
monster in the XML is `targetdistance="1"` (128 of them). So today, `perType` and a hardcoded `4`
produce **identical behavior** — there is no live bug here.

C++ — `crnonpl.cc:2837-2871`, the band is a **literal 4** for every distance fighter:
```cpp
if(Distance < 4){            ... SearchFlightField → ToDoGo(...) else ToDoWait(1000)
}else if(Distance > 4){      ... ToDoGo(Target->posx, ..., false, Distance - 4);
}else{                       ... rand()%5 sidestep, require DestDistance == 4, ToDoWait(1000);
```

Rust — `idle_stimulus.rs:2785,2800-2806` and `monster_ai.rs:64-77` use
`monster_effective_target_distance(m.target_distance)`, and `data/formulas/772.lua:15` selects
`distanceKeep = "perType"` → the band is whatever each monster's TFS XML `targetdistance` says.

**Why it still matters (structural, not numeric):** the decompile encodes **two independent facts** —
a boolean `DistanceFighting` race flag, and a band that is *always* the literal 4. Rust collapses both
into a single `target_distance` integer, where `> 1` means "is a distance fighter" and the value itself
means "band". Consequences:

- The two facts can no longer disagree. A 772 distance fighter that should keep 4 and a TFS monster that
  merely "keeps its distance" are indistinguishable.
- Any data edit or re-import that writes `targetdistance="3"` or `"5"` silently changes 772 mechanics —
  band, flee threshold, dance ring, *and* the `Distance - 4` chase step budget — with no error and no
  test failure. The invariant "the band is 4" is currently upheld only by the data pack happening to be
  correct.
- The one-off (`human` present in 772, absent in XML) shows the two sources already drift.

**Fix (low priority, hardening):** set `distanceKeep = 4` in `data/formulas/772.lua` (leave `perType`
for 1098) and carry an explicit `distance_fighting` bool imported from the `Flags` block, so the gate
and the band stop sharing a field. No behavior change today; it makes the invariant enforced instead of
coincidental.

---

### AI-4 No `MonsterhomeInRange` despawn inside `IdleStimulus`
**Severity: BUG**

C++ — `crnonpl.cc:2407-2416`, the `else` arm for non-summons:
```cpp
}else{
    if(!MonsterhomeInRange(this->Home, this->posx, this->posy, this->posz)){
        print(3, "%s an [%d,%d,%d] zu weit von [%d,%d,%d] entfernt.\n", ...);
        this->StartLogout(true, true);
        this->State = SLEEPING;
        return;
    }
}
```
with (`crnonpl.cc:1515-1529`) `|z-hz| <= 2 && |x-hx| <= Radius && |y-hy| <= Radius`.

Rust — `monster_idle_stimulus_inner` (`idle_stimulus.rs:1947-2061`) has **no** home-range branch. The
leash exists only as a *movement* filter in `monster_move_possible_planning`
(`monster_ai.rs:1786-1791`) plus the TFS-shaped `monster_maybe_walk_to_spawn` / `walkToSpawnRadius`
path.

**Impact:** a monster lured outside its Monsterhome (dragged by a player, pushed, or teleported) is never
removed. In 772 it despawns on its next idle pass. Long-term this leaks monsters outside spawns and
breaks respawn accounting.

---

### AI-5 `DamageStimulus` yields on every state change (772 yields only from SLEEPING)
**Severity: BUG**

C++ — `crnonpl.cc:2308-2321`. `ToDoYield()` appears **once**, inside the `SLEEPING` arm:
```cpp
if(this->State == SLEEPING){
    if(this->Target == 0) this->State = PANIC; else this->State = UNDERATTACK;
    this->ToDoYield();
}else{
    if(this->Target == 0)            this->State = PANIC;
    else if(this->State == IDLE)     this->State = UNDERATTACK;
}
```
No yield, no ToDo mutation, on the awake path.

Rust — `idle_stimulus.rs:630-636`:
```rust
if state_changed || was_sleeping {
    ... m.base.delay_attack_ms(self.server_ms, 4000);
    self.creature_todo_yield(victim_id);
}
```

**Impact:** an already-awake monster that takes a hit re-arms its ToDo immediately (`ToDoWait(0)` →
wakeup at `server_ms+1`) instead of finishing its current schedule. Monsters visibly react faster to
being hit than 772, and the extra idle passes shift every downstream RNG draw.

---

### AI-6 `DamageStimulus` injects a 4000 ms attack delay with no decompile counterpart
**Severity: BUG (invented constant)**

Rust — `idle_stimulus.rs:632-634`:
```rust
// First melee after damage lands on the second post-damage idle (`tick=4000` in panic sim).
m.base.delay_attack_ms(self.server_ms, 4000);
```

C++ `TMonster::DamageStimulus` (`crnonpl.cc:2304-2343`) touches **only** `State` and (from SLEEPING)
`ToDoYield`. It never touches `Combat.EarliestAttackTime`. The 2000 ms melee cadence comes from
`DelayAttack(2000)` in `crcombat.cc`, not from taking damage.

**Impact:** every state-changing hit pushes the monster's next melee out by 4 s. This looks like it was
back-fitted to one oracle trace; if that trace is real, the true cause is elsewhere (likely the extra
yield in AI-5 shifting the attack schedule) and this constant is masking it.

---

### AI-7 Casting: aggressive-gate runs before the impact roll → RNG stream divergence
**Severity: BUG (parity harness) / RISK (live)**

C++ — `crnonpl.cc:2586-2681`. The `TImpact` (and therefore `ComputeDamage(this, 0, Damage, Variation)`,
which consumes RNG) is constructed for **every** spell that passed the delay + flee rolls, and only
afterwards:
```cpp
if(!Impact->isAggressive() || (this->Target != 0 && this->Target != this->Master)){
```

Rust — `idle_stimulus.rs:1223-1231` evaluates the aggressive gate and `continue`s **before** any impact
value is rolled.

**Impact:** for a monster with aggressive spells and no valid target, C++ burns `ComputeDamage` draws and
Rust does not. Every subsequent glibc-parity draw in that beat diverges. Seeded oracle comparison will
drift on any caster that idles without a target.

---

### AI-8 `LifeEndRound` (wave/raid creature lifetime) is not implemented
**Severity: GAP**

C++ — `crnonpl.cc:2352-2357` (first thing after the LockToDo guard):
```cpp
if(this->LifeEndRound != 0 && this->LifeEndRound <= RoundNr){
    print(3, "Lebenszeit für %s abgelaufen.\n", this->Name);
    this->StartLogout(true, true);
    this->State = SLEEPING;
    return;
}
```
Set from raid waves at `crmain.cc:2063` (`RoundNr + Wave->Lifetime`) and inherited by summons at
`crnonpl.cc:2021`.

Rust — `grep -rn "life_end_round\|LifeEndRound"` over `crates/` returns nothing.

**Impact:** raid/wave-spawned creatures never expire.

---

### AI-9 Summon may not step into master adjacency — not ported
**Severity: GAP**

C++ — `TMonster::MovePossible`, `crnonpl.cc:2171-2180`:
```cpp
if(Execute && this->Master != 0 && this->State != ATTACKING && this->State != PANIC){
    TCreature *Master = GetCreature(this->Master);
    if(Master != NULL && Master->posz == this->posz){
        if((|Mx-posx| + |My-posy|) > 1 && (|Mx-x| + |My-y|) <= 1) return false;
    }
}
```
i.e. a non-fighting summon is forbidden from *entering* Manhattan-1 of its master.

Rust — no counterpart in `monster_move_possible_planning` / `monster_can_walk_to`.

**Impact:** summons crowd onto their owner's shoulder instead of holding at distance 2.

---

### AI-10 `SKILL_GO_STRENGTH < 0` move block not ported
**Severity: RISK (minor)**

C++ — `crnonpl.cc:2163-2167` refuses all movement when `Skills[SKILL_GO_STRENGTH]->Act < 0`.
No equivalent guard in the Rust move gate. Only reachable via over-application of paralyze.

---

### AI-11 Melee dance "no step" case does not emit a `ToDoGo`
**Severity: RISK**

C++ — `crnonpl.cc:2803-2827`: on `rand()%5 == 4` the destination stays the current tile, `DestDistance`
is still 1, and if `MovePossible(own tile)` succeeds it still calls
`ToDoGo(DestX, DestY, DestZ, true, INT_MAX)` — a Go to the tile it is standing on, which consumes a ToDo
action and a `CalculateDelay`/`EarliestWalkTime` slot.

Rust — `monster_ai.rs:1072-1080` pushes nothing on the `None` direction and the branch resolves to
`Hold` (`idle_stimulus.rs:3010-3028`).

**Impact:** the 1-in-5 "stand still" beat is paced differently from 772 (no walk-delay consumption).

---

### AI-12 Casting range pre-check + rotate ordering
**Severity: RISK**

Rust — `idle_stimulus.rs:1281-1283` skips the spell entirely when `dist > spell.range`, before any
`Rotate`. C++ has **no** range test in the CASTING block; range lives inside `VictimShapeSpell`
(`magic.cc:423`) / `CircleShapeSpell` (`magic.cc:520`), and `AngleShapeSpell` has none at all — and
`this->Rotate(Target)` (`crnonpl.cc:2691, 2712, 2725`) runs *before* the shape call regardless.

**Impact:** out-of-range casts in 772 still turn the monster toward its target (visible 0x6B) and, for
`SHAPE_ANGLE`, still fire the cone. Rust suppresses both.

---

### AI-13 `DamageStimulus` gate: `damage <= 0` vs C++ `Damage != 0`
**Severity: RISK (minor)**

C++ `crnonpl.cc:2305`: `if(AttackerID != 0 && Damage != 0)` — **negative** damage (healing) also drives
the state machine, and there is no self-exclusion. Rust `idle_stimulus.rs:570` returns on
`damage <= 0 || attacker_id == victim_id`.

---

### AI-14 Home-range z-band and center
**Severity: RISK (minor)**

C++ `MonsterhomeInRange` hardcodes `|z - MH->z| <= 2` around the **Monsterhome record**, and
`MovePossible` applies a *second* `max(|x-posx|,|y-posy|) > this->Radius` test relative to the monster's
current position (`crnonpl.cc:2153-2158`). Rust uses the creature's `spawn_position` and the configurable
`monster_world_config.despawn_z_range` (`monster_ai.rs:1788`), and has no second test.

---

### AI-15 Monster `LoggingOut` guard absent
**Severity: RISK (minor)**

C++ `crnonpl.cc:2346`: `if(this->LockToDo || this->LoggingOut) return;`. `logging_out` exists only on
`Player` in Rust; monsters despawn synchronously, so a monster mid-despawn can still run one idle pass.

---

### AI-16 `CreatureMoveStimulus` has no `Type != OBJECT_DELETED` discriminator
**Severity: RISK**

C++ gates every wake on `Type != OBJECT_DELETED` (`crnonpl.cc:2945, 2952, 2962`). Rust
`monster_sleep_wake_on_creature_move` (`idle_stimulus.rs:654-693`) takes no move-type parameter, so the
distinction depends entirely on the call sites in `monster_events.rs` / `walk/mod.rs` not invoking it on
removal. Worth an explicit parameter rather than a call-site convention.

---

## 2. ToDo scheduler — findings

### TD-1 `TDTrade` is not modeled
**Severity: GAP**

C++ — `enums.hh:677-688`:
```cpp
enum ToDoType: int { TDWait=0, TDGo=1, TDRotate=2, TDMove=3, TDTrade=4,
                     TDUse=5, TDTurn=6, TDAttack=7, TDTalk=8, TDChangeState=9 };
```
`TCreature::ToDoTrade` at `cract.cc:1202`, dispatched from `receiving.cc:327`, executed at
`cract.cc:828-831`.

Rust — `CreatureAction` (`creature_todo.rs:112-153`) has no `Trade` variant. Player↔player trade does not
go through the ToDo queue (walk-to-partner, 1-tile range, and the `Wait` pacing are therefore absent).

### TD-2 `TDTalk` drops `Mode`, `Addressee` and `CheckSpamming`
**Severity: GAP**

C++ `Execute` (`cract.cc:848-857`) calls
`Talk(this->ID, TD.Talk.Mode, Addressee, Text, TD.Talk.CheckSpamming)`.
Rust `CreatureAction::Talk { text }` (`creature_todo.rs:124`) and its execute arm
(`idle_stimulus.rs:3163-3179`) hardcode `MonsterSay` / `Say`, pass no addressee and never spam-check.
Queued yells (`TALK_ANIMAL_LOUD`) and NPC addressed replies degrade to plain say.

*(Note: the monster **idle** talk path handles `#y` yells correctly at `idle_stimulus.rs:2159-2175` — this
gap only affects queued `TDTalk`.)*

### TD-3 `ToDoClear` does not run pending `TDChangeState` for NPCs
**Severity: BUG**

C++ `ToDoClear` (`cract.cc`, quoted in full during this audit):
```cpp
case TDChangeState:{
    if(this->ActToDo <= i && this->Type == NPC){
        ChangeNPCState(this, TD->ChangeState.NewState, false);
    }
    break;
}
```
Rust `player_todo_clear` (`creature_todo.rs:961-975`) clears the deque and walk state only. A cleared
NPC that had a pending `ChangeNpcState` stays in its old activity/focus instead of being forced to the
queued state without stimulus.

### TD-4 `process_creature_todo` consumes `next_wakeup`; C++ `Execute` does not
**Severity: RISK**

C++ `Execute` (`cract.cc:783-787`) only *tests* `NextWakeup > ServerMilliseconds`; the field is
overwritten by the next `ToDoStart`. Rust takes it (`walk/mod.rs:491-497`
`base_mut().next_wakeup.take()`), so if a creature has two heap entries due in the same beat the second
pop is a no-op in Rust but re-enters `Execute` in C++ (usually a fast `!LockToDo` break — hence RISK, not
BUG). The drain's `due` test at `walk/mod.rs:457-461` is otherwise a correct reading of `cract.cc:785`.

### TD-5 `ToDoMove` creature-container branch (Delay = 1000 + walk cooldown) not ported
**Severity: GAP (already documented as "D9")**

C++ `cract.cc:1143-1160`. Rust `enqueue_player_move` (`creature_todo.rs:571-585`) always uses the flat
`ToDoWait(100)`.

### TD-6 `TDRotate` intentionally omitted
**Severity: GAP (accepted)**

Documented at `creature_todo.rs:105-110` — 772's idle combat tail calls `Rotate(Target)` directly
(`crnonpl.cc:2871-2873`), so the queued form is not needed and enqueuing it produced a visible
turn-on-the-spot. Keep, but keep the note.

---

## 3. Verified correct (spot-checked against the C++ this pass)

Scheduler:
- `todo_queue.rs` **is** a faithful port of `containers.hh` `priority_queue::insert` / `deleteMin`
  (1-indexed, `Parent->Key <= Current->Key` stop, strict left-child bias on ties).
- `drain_todo_queue` matches `MoveCreatures` (`crmain.cc:1142-1158`): advance `ServerMilliseconds`, pop
  while `ExecutionTime <= server_ms`, skip missing creatures.
- `CalculateDelay` — all four live cases (`TDWait` with `EarliestWalkTime` floor, `TDGo`, two-object
  `TDUse` with `EarliestMultiuseTime`, `TDAttack` with `max(EarliestAttackTime, EarliestSpellTime)`)
  match `cract.cc` exactly.
- `ToDoStart` `Delay < 1 → 1` clamp, `LockToDo` semantics, `Stop` flag, `ToDoYield = ToDoWait(0)+Start`.
- `AdvanceGame` lag guard (`Delay >= 1000` skips `MoveCreatures`).
- `ToDoClear` snapback: Rust's `has_go()` over the remaining deque **is** equivalent to C++
  `ActToDo <= i`, because executed entries are `pop_front`ed (`idle_stimulus.rs:3123`). (An earlier audit
  pass flagged this as a divergence — it is not.)

Monster AI:
- Strategy roll arithmetic, incl. "last bucket is random" (`monster_idle_roll_strategy_from_roll`).
- Target-acquisition scan: 12×12 box, `FIND_PLAYERS|FIND_MONSTERS` (NPCs excluded), wild monsters
  `continue`d **before** the `CanSeeFloor` sleep check (matches `crnonpl.cc:2500-2506` — an earlier audit
  pass wrongly called this a bug), same-z + ≤10 axis box, PZ/house/`IGNORED_BY_MONSTERS`/invisible
  filters, `random(0,99)` tie-break with `>` comparison, `ShouldSleep → SLEEPING` with no `ToDoStart`.
- Lose-target predicate order and master-gating (`crnonpl.cc:2420-2434`).
- Talk: `rand()%50`, `random(1,Talks)` 1-indexed, `#y`/`#Y` + trailing-space prefix → `TALK_ANIMAL_LOUD`.
- Summon lifecycle constants: master gone / non-player master off-floor / `|Δz|>1` / `|Δx|,|Δy|>30`, and
  player-master → logout vs monster-master → kill (`crnonpl.cc:2359-2405`).
- Summon casting: only wild masters build `TSummonImpact` (`crnonpl.cc:2647`), `SummonedCreatures < Max`.
- Flee branch failure falls through to the random-roam tail (`idle_stimulus.rs:3073-3079` ≡
  `crnonpl.cc:2884-2942`); roam is 10 × `rand()%4` in W/E/N/S order.
- `KickCreature` / `KickBoxes` 100-attempt retry loop and the player-tile `Target = 0; throw EXHAUSTED`
  drop (`crnonpl.cc:2184-2240` ↔ `monster_push.rs`).
- PANIC → ATTACKING promotion after a successful melee dance (`crnonpl.cc:2830`).

---

## 4. Suggested priority

| # | Finding | Affected races | Effort |
|---|---------|---|--------|
| 1 | **AI-2** — import `LoseTarget` from `runtime/mon/*.mon` | **90 / 159** | S |
| 2 | **AI-1** — import `Strategy = (a,b,c,d)` from the same files | **63 / 159** | S |
| 3 | AI-5 / AI-6 — restrict `ToDoYield` to the SLEEPING arm, delete the 4000 ms delay, re-run the panic oracle | all | M |
| 4 | AI-4 — home-range despawn branch at the top of `monster_idle_stimulus_inner` | all | S |
| 5 | AI-7 — roll the impact value before the `isAggressive` gate | casters | S |
| 6 | AI-8 — `life_end_round` on `CreatureBase` + wave lifetime | raids | M |
| 7 | TD-3 — run pending `TDChangeState` in the NPC clear path | NPCs | S |
| 8 | AI-9 / AI-11 / AI-12 — small `MovePossible` and cast-ordering fixes | few | S |
| 9 | TD-1 / TD-2 — `TDTrade`, `TDTalk` mode/addressee | players | L |
| 10 | AI-3 — `distanceKeep = 4` + explicit `distance_fighting` flag (hardening, no behavior change) | 29 | S |
| 11 | Un-ignore `reference/**` for search tooling | — | S |

**Note on #1–#2:** these are not "figure out the right numbers" tasks. Every value is sitting in
`reference/cipsoft-772/runtime/mon/*.mon` in a trivially parseable `Key = Value` format, keyed by
`Name`, alongside `FleeThreshold`, `Attack`, `Defend`, `Armor`, `Poison`, `SummonCost` and the `Flags`
block. A single importer generating a 772 overlay table closes AI-1, AI-2 and the AI-3 hardening at
once, and is worth auditing against the rest of `MonsterAiConfig` for further silently-defaulted fields.

## 5. Pathfinding (`TShortway`) — follow-up pass

Audited `pathfinding.rs` against `cract.cc:7-305` (`TShortwayPoint`, `TShortway::{FillMap, ClearMap,
Expand, Calculate}`). **This is the most faithful subsystem in the audit.** Verified identical:

- Matrix spans `±(Visible+1)` with only the inner `±Visible` terrain-filled; outer ring stays
  `Waypoints = -1` (`pathfinding.rs:743-770` ↔ `cract.cc:68-70, 80-116`).
- `VisibleX/Y` = 7 players / 10 monsters (`REVERSE_PATH_VIEW_RADIUS`, `PLAYER_PATH_VIEW_RADIUS`).
- `FillMap`: stack-head `BANK && !UNPASS` → `WAYPOINTS`, then `MovePossible(Execute=false)` → `-1`;
  `MinWaypoints` seeded at **1000** and only lowered by tiles that pass *both* gates
  (`pathfinding.rs:771-795` ↔ `cract.cc:89-106`).
- Reverse expansion dest→origin, seed `Waylength = 0`, run until the expand list drains.
- Edge cost is the **leave tile's** `Waypoints`, diagonal = ×3 via `+ Waypoints * 2`
  (`pathfinding.rs:670-674` ↔ `cract.cc:174-178`).
- Node-level branch-and-bound `MinNeighborWaylength >= Map->at(0,0)->Waylength → return`.
- Heuristic `Waylength + Waypoints + MinWaypoints * (Distance - 1)` with **Manhattan** `Distance`.
- Open set is a heuristic-sorted singly-linked list, inserting **before** the first `>=` entry
  (`cur_h < new_h` advance) — so equal-heuristic ties are LIFO, matching `cract.cc:210-222`.
- Re-relaxed nodes are unlinked and re-inserted (`remove_from_expand_list`).
- Neighbor iteration order is exactly C++'s `OffsetX` outer / `OffsetY` inner
  (`REVERSE_PATH_NEIGHBOR_OFFSETS`, `pathfinding.rs:1228-1237`).
- Enqueue gate `(x,y) != origin && Waypoints != -1` (Rust `<= 0`, equivalent after FillMap
  normalizes `0 → -1`).
- Out-of-viewport target → no path; already-at-target → trivial success.
- Reconstruction + truncation: `MaxSteps > 0 && (MustReach || CurDistance > 1)`, with `CurDistance`
  seeded from the origin and recomputed from the tile **just appended**
  (`truncate_tshortway_go_queue`, `pathfinding.rs:1377-1403` ↔ `cract.cc:271-296`).

### PATH-1 Extra per-neighbor prune not present in C++
**Severity: RISK**

`pathfinding.rs:675-677`:
```rust
if neighbor_wl >= origin_wl_i { continue; }
```
C++ `Expand` prunes only once, at node level (`cract.cc:159-162`); each neighbor is then relaxed on
`NeighborWaylength < Neighbor->Waylength` alone. A neighbor whose waylength already exceeds the
origin's still gets its `Waylength`/`Predecessor` written and can be enqueued in 772.

Cannot lengthen the final path (predecessor chains from the origin are strictly decreasing), but it
**does** change which nodes enter the expand list and in what order → different tie-breaking between
equal-cost paths. Expect occasional "took the other equally-short route" divergence vs the oracle.

### PATH-2 Expansion cap has no C++ counterpart
**Severity: RISK (low)**

`pathfinding.rs:823-826` breaks at `viewport_closed_cap(radius) * 2` (≈882 expansions for monsters).
`TShortway::Calculate` runs `while(this->FirstToExpand != NULL)` with no bound; because relaxed nodes
are re-inserted, 772 can exceed that count on cheap-terrain fields. Hitting the cap yields a *shorter*
or missing path where 772 finds one. Safety guard is fine — but it should log when tripped, like the
`drain_todo_queue` runaway guard does.

### PATH-3 `allow_diagonal` knob has no 772 equivalent
**Severity: RISK (low)**

`expand()` filters diagonals on `fpp.allow_diagonal` (`pathfinding.rs:658`); `TShortway::Expand` always
walks all 8. Default is `true` and monster callers use the default, so this is currently inert — but it
is a foot-gun that silently produces non-772 paths if ever set false on the reverse path.

### PATH-4 Debug-trace `MinWaypoints` uses a different predicate than the search
**Severity: cosmetic**

`monster_ai.rs:1218` / `:1281` log `scan_min_terrain_waypoints(...)`, which gates on
`map.is_walkable(pos)` and falls back to `DEFAULT_TERRAIN_WAYPOINTS` (150) when nothing is found. The
real search gates on the per-creature `can_walk_to` (`MovePossible`) and seeds 1000
(`pathfinding.rs:749, 771-795`). The logged `min_wp` can therefore disagree with the value actually used
— confusing when diffing chase traces against the oracle.

### Caveat: correct pathfinder ≠ correct pathing
The `TShortway` **search** is right; what is fed to it is where behavior diverges. AI-3 (band 4 vs
per-type `targetdistance`) directly changes both the `dist_chase` goal and its `Distance - 4` step
budget, and AI-5/AI-6 change how often a repath is requested at all. Fix those before judging chase
behavior against a 772 recording.

---

## 6. Verification

```bash
rtk cargo check -p tfs-rust-core
rtk cargo clippy -p tfs-rust-core -- -D warnings
rtk cargo test -p tfs-rust-core idle_stimulus
rtk cargo test -p tfs-rust-core monster_ai
rtk cargo test -p tfs-rust-core todo_queue
```

Suggested new tests per fix: strategy-bucket selection from parsed data (AI-1), lose-target roll at
`losetarget = 50` (AI-2), dist-band flee/dance/chase at literal 4 (AI-3), no yield on IDLE→UNDERATTACK
(AI-5), idle despawn outside home radius (AI-4), RNG-draw count for an aggressive spell with no target
(AI-7).
