# 772 Monster AI — High-Level Parity Audit

**Status Update (Jun 2026 re-verification):** Most critical findings have been addressed. Findings 16 (LOS `ThrowPossible`) and 17/17b (chase leash) are now **FIXED**. The remaining "feel" defects cluster around **melee smoothness**: the atomic `Execute` loop (game-loop Finding 7 — still one-action-per-pop), the Rotate gate (AI#22), the missing kick-and-retry loop (AI#23), and the Z-level target clear (AI#24). See "Smoothness Root Cause Analysis" and "Implementation Status Summary" at end.

Scope: a *structural* pass over the Rust monster AI against the CipSoft 772 decompile
(`reference/cipsoft-772/tibia-game-master/src/`). Focus areas: idle stimulus, monster walking,
the map/viewport (Pass 1), and melee reactivity / bumping / the step-scheduling layer (Pass 2).
This is **not** a line-by-line trace — it flags places where the Rust shape diverges from the
reference in ways that change observable behavior or the RNG draw sequence (the usual cause of
"passes the sim but feels off").

Reference functions:
- `TMonster::IdleStimulus` — `crnonpl.cc:2386–2945`
- `TMonster::MovePossible` / `KickCreature` — `crnonpl.cc:2141–2291`
- `TShortway::Calculate` / `TCreature::ToDoGo` / `NotifyGo` — `cract.cc`
- `MoveCreatures` / main loop — `crmain.cc:1142`, `main.cc:446/496`, `config.cc:102`

Rust under audit: `idle_stimulus.rs`, `monster_ai.rs`, `monster_distance_step.rs`,
`creature_todo.rs`, `walk/`, `pathfinding.rs`, `map/`, `monster_push.rs`, `game_loop.rs`,
`todo_queue.rs`, `game_world_tick.rs`.

---

## Verdict (Pass 1)

The overall architecture is faithful and well-referenced:

- **Idle trigger** is drain-based (`idle_stimulus` fires when the per-creature `CreatureTodo`
  queue empties), matching 772 `ToDoStart`/`ToDoYield` semantics rather than TFS 1098's
  `onThink` arming. Correct.
- **Idle phase ordering** — lose-target → state normalize → talk → target acquire → cast →
  walk → roam fallback — matches `crnonpl.cc` and preserves the RNG draw order with explicit
  trace sites.
- **Walk timing** uses the linear `GoStrength` model with the diagonal(×3)/floor(×2) waypoint
  costs — matches `NotifyGo`.
- **Path search** uses the reverse `dest→origin` `TShortway` with `VisibleX/Y = 10` and the
  ~441-node cap — matches `cract.cc`.
- **Melee chase budget** `CHASE_PATH_MAX_STEPS = 3` matches `ToDoGo(..., false, 3)`.

Why it still "feels off": the divergences below. Pass 1 found two in the decision tree
(distance band, casting). Pass 2 found the bigger one for *melee* — the monster-vs-monster push
layer is ported from the wrong era and uses a non-deterministic RNG.

---

## Findings (ranked by likely "feel" impact)

### 1. Distance-fighting keep band: hardcoded `4` in 772 vs per-type `targetDistance` in Rust  — RECLASSIFIED (no change needed for shipped data)

> **Update (Phase 4 reassessment):** no code change. The shipped `data/monster/` set uses
> `targetdistance="4"` for **all** distance monsters and `"1"` for all melee (29× `4`, 128× `1`, no
> other value), so `DistanceKeep::PerType` resolves to the exact C++ literals and is behaviorally
> identical to the hardcode. Pinning `Fixed(4)` is **not** recommended — `target_distance` doubles as
> the melee-vs-distance discriminator, so a global 4 would mis-classify melee monsters that carry a
> ranged spell. The only residual exposure is content drift (a distance monster authored with
> `targetdistance != 4`); an optional warn-only load guard covers that. See Phase 4 below.

In `crnonpl.cc:2795–2834` the distance-fighting branch is built around a **hardcoded chebyshev
distance of 4** for *every* distance monster:

```cpp
if (Distance < 4)      { ... SearchFlightField ... }            // dist_flee
else if (Distance > 4) { ToDoGo(target, false, Distance - 4); } // dist_chase
else /* == 4 */        { rand()%5 sidestep; ToDoWait(1000); }   // dist_dance
```

772 `RaceData` has no per-type keep distance — the band is a global constant.

Rust uses the per-type `targetDistance` (from `monsters.xml`) as the band, and the 772 profile
ships `distanceKeep = "perType"` (`data/formulas/772.lua`, `DistanceKeep::PerType`):
- `monster_idle_classify_walk_branch` → `dist < / == / > target_distance`
- `monster_idle_chase_step_budget` → `cheb - target_distance`
- `monster_at_follow_goal` → `dist == target_distance`

**Latent effect (only if data drifts):** a distance monster whose XML `targetDistance ≠ 4` would keep
at the wrong range, chase/flee with the wrong step budget, and dance at the wrong ring vs 772. With
the shipped uniform-4 data this never triggers.

### 2. Casting loop stops after the first spell; 772 evaluates every spell  — HIGH

`crnonpl.cc` CASTING block (`~2521–2667`) loops `for SpellNr = 1..=Spells`, each gated by its
own `rand() % Delay`, with **no `break`** — the decompiler even comments "we could cast all
spells at once ... I can see it happening." So a multi-spell 772 monster can fire several
impacts in one idle, and **draws a delay roll for every spell every idle**.

`idle_stimulus.rs::monster_idle_try_casting` breaks after the first successful cast:

```rust
if cast_any { break; }
```

**Effect (behavioral):** monsters with 2+ offensive spells cast at most one per idle in Rust.
**Effect (RNG):** when an early spell fires, the remaining offensive spells' `parity_rand_mod`
draws are skipped (the tail loop only consumes *defense* spell delays), so the glibc RNG stream
**desyncs** from C++ for the rest of that creature's life. Single-spell monsters (most sim
fixtures) never hit this.

**Recommendation:** remove the `break`; evaluate and roll every spell like C++, casting each
that passes its gate.

### 3. Monster `Talk` is RNG-only — no packet emitted  — LOW (cosmetic, verify)

`monster_idle_try_talk` consumes the `rand()%50` gate + `random(1,Talks)` pick (correct for RNG
parity) but never emits the talk. 772 `Talk(this->ID, Mode, ...)` (`crnonpl.cc:2393`) actually
broadcasts monster yells. Fine for the sim harness; confirm a live 772 server still produces
monster talk.

### 4. Dance / roam distribution — confirm (no divergence spotted)

- `melee_dance` / `dist_dance` should use `rand()%5` (4 cardinals + a no-op "stay"), step taken
  only if `MovePossible`. Confirm `monster_idle_dance_step` keeps the no-op as 1-in-5.
- Roam: 772 tries `rand()%4` cardinals up to 10×, then `ToDoGo + ToDoWait(1000)`. Confirm the
  Rust roam keeps the 10-try cap and cardinal-only set.

---

# Pass 2 — Melee reactivity, bumping, and the step/scheduling layer

Follow-up focused on melee monsters "bumping into each other, stopping for a second, feeling
clunky," and the suspicion that the cause is "further up the path / in the thread." Summary:
**the macro loop cadence is faithful; the clunk is in the monster-vs-monster push layer, which
is ported from the wrong era and uses a non-deterministic RNG.**

## What is NOT the problem (ruled out with C++ refs)

### Loop cadence / "the thread" — faithful

The 772 beat loop matches CipSoft:

- `Beat = 200` ms — `config.cc:102` (`Beat = 200;`).
- Main loop: `AdvanceGame(NumBeats * Beat)` — `main.cc:496`; `MoveCreatures(Delay)` does
  `ServerMilliseconds += Delay; while(ToDoQueue top.Key <= ServerMilliseconds) Execute();`
  — `crmain.cc:1142–1158`.
- Rust mirrors this exactly: `advance_beat_772(beat_ms=200)` → `server_ms += 200` →
  `drain_todo_queue()` pops all entries with `execution_time <= server_ms`
  (`game_world_tick.rs:41`, `walk/mod.rs:332`, `todo_queue.rs`).

So logical time advancing in 200 ms steps and draining the heap per beat is **correct** — both
engines quantize creature scheduling to a 200 ms grid. The "thread" instinct points at the right
layer (step scheduling, not the decision tree) but the loop itself is not the defect.

### Step-duration quantizer — latent, observably masked

`NotifyGo` in CipSoft quantizes `EarliestWalkTime` to a multiple of **`Beat` (200 ms)**:

```cpp
int Delay = (Waypoints * 1000) / this->GetSpeed();
int BeatCount = (Delay + Beat - 1) / Beat;                       // ceil to 200
this->EarliestWalkTime = ServerMilliseconds + BeatCount * Beat;  // cract.cc:1531-1534
```

Rust's `walk/walk_timing.rs` quantizes to `step_beat_ms = 50 ms` (`walk_quantizer_ms`), citing
the TVP `gameserver` wire animation grid rather than the CipSoft mechanics authority. Per the
era rules this is the wrong source (772 **mechanics** = `tibia-game-master/src/`; TVP
`gameserver` is the **wire** reference only). But because the live loop only drains on 200 ms
boundaries, `ceil(V/50)*50` and `ceil(V/200)*200` collapse to the same fire time (50 divides
200). So this is a **latent authority mismatch**, not the felt defect. (Note: it is *not* masked
in the harness, where `server_ms` can be set to non-200 values — which can make sim logs disagree
with a live server.)

## The actual cause — monster-vs-monster push is the wrong model + non-deterministic

### Finding 5. `monster_push.rs` is ported from TFS 1098, not CipSoft 772  — HIGH

`monster_push.rs` headers cite `Monster::pushCreature/pushCreatures` (`monster.cpp`, the **1098**
tree) and implement the 1098 model: push every blocker off the destination tile *before* the
mover steps, random-cardinal `internalMoveCreature`, kill the blocker if the push fails.

CipSoft 772 does something structurally different in `TMonster::MovePossible`
(`crnonpl.cc:2141–2291`):

- A blocking creature only yields when **all** of: `State == ATTACKING || PANIC`, `Target != 0`,
  the mover's race has `KickCreatures`, the blocker is not the Target/Master, not `Unpushable`,
  not an NPC, and (for players) the mover has no master and the player isn't `IGNORED_BY_MONSTERS`.
- **Planning** (`Execute == false`): a kickable creature tile is treated as *plannable-through*
  (the loop falls past it) — so the path may route across a pushable ally.
- **Execution** (`Execute == true`): the mover calls `KickCreature(blocker)` to shove it aside
  and then *retries the same tile* (up to 100 attempts, also kicking boxes/`UNPASS`/`AVOID`
  fields). If the kick fails it `throw EXHAUSTED`. Players are never kicked — hitting a player
  tile clears `Target` and throws `EXHAUSTED`.

Consequences of the mismatch:

- Monsters **without** `KickCreatures` (most of them — `amazon`, `war wolf`, `fire devil`,
  `mummy`, `orc warrior`, `chicken`, … all `canpushcreatures="0"`) should treat each other as
  **hard blockers** and route around in `TShortway`. The 1098 push path doesn't gate the same way.
- Gating on `ATTACKING/PANIC + Target` is absent in the Rust push entry
  (`monster_push_before_step` only checks `can_push_creatures && !is_summon`), so push happens in
  states where 772 would not, and vice-versa.
- **Kill semantics differ from 1098, but a kill still happens.** 1098 kills a blocker when the
  *mover's* push fails and then the mover proceeds. 772 `KickCreature` (`crnonpl.cc:3074-3080`)
  kills a **boxed-in pushable monster** (no free adjacent N,S,W,E tile) by dealing damage equal to
  the victim's **full current HP attributed to the kicker** (`AddDamageToCombatList(this->ID, …)`
  + `Kill()`) — so kill credit, corpse, loot and experience all go to the kicker — and the mover
  then throws `EXHAUSTED` and waits 1000 ms (it does **not** step onto the cleared tile this beat;
  see Finding 7). Players are never killed this way (a player tile is the `Target=0` + `EXHAUSTED`
  case, no kick).
  **Status (Phase 1, implemented):** `monster_kick_creature_772` mirrors this exactly — full-HP
  physical damage via `combat::execute(Some(kicker), …)` (records the kicker in the victim's
  `damage_map`) + `EFFECT_BLOCK_HIT` + `apply_creature_death` (corpse/loot/exp). It calls the
  death path directly rather than `combat_execute_with_stimulus` because C++ `Kill()` does not
  re-run `DamageStimulus`.

### Finding 6. Push direction uses `rand::thread_rng()` — non-deterministic, breaks RNG parity  — HIGH

```rust
// monster_push.rs::monster_push_creature
dirs.shuffle(&mut rand::thread_rng());
```

Every other AI draw in the engine goes through `sim_glibc_rand::parity_*` (the glibc-equivalent
stream the whole 772 parity effort is built on). This push path uses the thread RNG instead, so:

- The shove direction is **non-reproducible** run-to-run — "they don't react the same" is exactly
  this. Two identical fights diverge as soon as a push happens.
- It draws from outside the tracked stream, so **every subsequent `parity_*` draw for that
  creature desyncs** from the C++ oracle. Sims that don't trigger a push pass; the moment two
  melee monsters contend for a tile, parity is lost.

This single line is the most likely source of both "feels different each time" and the downstream
"passes the sim but is subtly wrong."

### Finding 7. Blocked-step recovery doesn't match the `EXHAUSTED` → `ToDoWait(1000)` contract  — MEDIUM

On a rejected step, Rust (`walk/mod.rs` ~1352) clears `walk_queue`, sets
`force_update_follow_path`, strips queued `Go`s, and calls `request_idle_stimulus` (yield
`Wait(0)` → re-plan on the same beat).

CipSoft splits this by *why* the step failed (`MovePossible` + the `IdleStimulus` catch,
`crnonpl.cc:2918–2926`):

- Kickable blocker → `KickCreature` + retry the **same** tile in the same `Execute` (no replan,
  no stall). This is what keeps melee packs flowing through each other.
- `EXHAUSTED` (kick failed / hit a player tile / `KickBoxes` failed) → the `IdleStimulus` catch
  does `Target = 0; ToDoClear(); ToDoWait(1000); ToDoStart();` — a deliberate **1-second** stop.

So 772 *does* "stop for a second," but only on genuine `EXHAUSTED`, and it **keeps moving** (via
kick-and-retry) in the common ally-contention case. The Rust path has no kick-and-retry, so the
common case degrades into clear-queue → yield → re-plan, which on the 200 ms grid reads as a
stutter, while the real 1000 ms `EXHAUSTED` wait isn't reproduced for the right trigger.

## Where to look / fix order

1. **Re-point `monster_push.rs` at `crnonpl.cc:2141` `MovePossible` + `KickCreature`** (772
   mechanics authority), not `monster.cpp`. Gate on `State ∈ {ATTACKING, PANIC}`, `Target != 0`,
   and the mover's `KickCreatures` flag; never kick Target/Master/NPC/Unpushable; clear-target +
   `EXHAUSTED` on a player tile.
2. **Replace `rand::thread_rng()` with the `sim_glibc_rand` parity stream** for the kick/shove
   direction, with a trace site, like every other AI draw. Confirm the C++ `KickCreature`
   direction-selection order first and mirror it — it may not be a uniform shuffle.
3. **Implement kick-and-retry-same-tile** at execution instead of clear-queue+replan, and route
   the real `EXHAUSTED` case to `Wait(1000)` (`MONSTER_IDLE_WAIT_MS`).
4. Verify `TShortway` FillMap treats non-`KickCreatures` movers' creature tiles as hard blockers
   so they route around each other (`monster_tshortway_fill_walkable` already gates on
   `can_push_creatures` — confirm the state/Target gating matches `MovePossible`).
5. Only after the above, revisit the `step_beat_ms` (50→200) authority fix for cleanliness, and
   the Pass-1 findings (distance band, casting loop).

## Repro suggestions

- Two `canpushcreatures="0"` melee monsters (e.g. two `war wolf`) on one player — compare tile
  occupancy with the C++ oracle; expect Findings 5/7 as bump-stall.
- Two `canpushcreatures="1"` monsters (e.g. `dwarf guard`) — run the same harness fight twice;
  Finding 6 should make the two Rust runs diverge from each other.
- A distance monster with `targetDistance ≠ 4` — exposes Finding 1.
- A two-offensive-spell monster — diff cast counts + post-idle RNG call count for Finding 2.

## Reference index

| Behavior | C++ 772 ref | Rust |
|---|---|---|
| Idle dispatch on queue drain | `cract.cc` `ToDoStart`, `crnonpl.cc:2386` | `idle_stimulus.rs::idle_stimulus` |
| Lose target / talk / strategy | `crnonpl.cc:2419/2440/2456` | `monster_idle_772_*`, `monster_idle_try_talk` |
| Casting | `crnonpl.cc:2521-2667` | `monster_idle_try_casting` |
| Distance band (const 4) | `crnonpl.cc:2795-2834` | `monster_effective_target_distance` (per-type) |
| Chase path | `cract.cc TShortway::Calculate` | `pathfinding.rs` reverse `TShortway` |
| Beat constant | `config.cc:102` (`Beat = 200`) | `formulas.rs` `beat_ms = 200` |
| Main loop drain | `main.cc:496`, `crmain.cc:1142` `MoveCreatures` | `game_world_tick.rs:41`, `walk/mod.rs:332` |
| Step duration quantize | `cract.cc:1531-1534` (`Beat`) | `walk/walk_timing.rs` (`step_beat_ms=50`) |
| Push / kick | `crnonpl.cc:2141` `MovePossible` + `KickCreature` | `monster_push.rs` (1098 model + `thread_rng`) |
| Blocked-step / EXHAUSTED | `crnonpl.cc:2918`, `MovePossible` throw | `walk/mod.rs` ~1352 clear+yield |

*Audit only — no AI code modified. Line-level RNG tracing of the push/kick path is the follow-up.*


---

# Pass 3 — RNG architecture (the deepest gap) + flee/roam/push/kick internals

Pass 2 found the push layer was the wrong era. Tracing it to the bottom exposed a more
fundamental issue that explains "they don't react the same" across the board: **the 772 AI draws
from three incompatible RNG streams, while CipSoft draws everything from one interleaved glibc
stream.** This is almost certainly the primary "feel" defect, and several Pass 1/2 findings are
symptoms of it.

## Finding 8. RNG stream fragmentation — `ai_rng` (StdRng) vs glibc-parity vs `thread_rng`  — CRITICAL

CipSoft uses glibc `rand()` / `random(min,max)` for **everything** — targeting, talk, spell
gates, damage (`ProbeValue`, `ComputeDamage`), armor (`GetArmorStrength`), poison, flee shuffles,
dance, roam — all from the **same** global stream, in a fixed interleaved order per idle.

The Rust engine has **three** streams:

| Stream | Backing | Used for |
|---|---|---|
| `sim_glibc_rand::parity_*` | `libc::srand` + glibc `rand()`/`random()` | lose-target, talk gate/pick, strategy roll, target tie-breakers, spell delays, dance choice (`sim_dance_choice`), melee realign |
| `ai_rng` | `StdRng` (ChaCha), `seed_from_u64` | **roam direction, flee `SearchFlightField` shuffles, melee/spell damage, defense roll, armor, poison** |
| `rand::thread_rng()` | OS entropy, non-deterministic | **monster push/kick direction** (Finding 6) |

`game_world.rs`: `ai_rng: StdRng`, `init_sim_rng_from_env` does `ai_rng = StdRng::seed_from_u64(seed)`
**and** `libc::srand(seed)` — two different generators seeded from the same number. ChaCha and the
glibc additive-feedback LCG produce **completely unrelated sequences**, so:

- Every `ai_rng` draw yields a value unrelated to what C++ produced (damage, flee, roam differ).
- Every `ai_rng` draw **does not advance** the glibc stream, so the glibc-parity stream drifts out
  of phase with the C++ oracle as soon as any damage/flee/roam draw happens between two
  glibc-critical draws. The codebase already fights this with the `TFS_SIM_MELEE_REALIGN` /
  `resync_harness_glibc_rng_from_env` hack in `monster_do_attacking` — direct evidence the streams
  desync and are being manually re-pinned.
- `thread_rng` in push is non-deterministic run-to-run — two identical fights diverge the instant
  a push happens.

**Why sims still pass:** the parity tests assert on the glibc-stream trace sites and use fixed
scenarios; `ai_rng` outcomes are either not asserted or seeded to a value that "looks plausible."
The moment real play involves flee, roam, multi-hit combat, or a push, behaviour diverges from the
decompile — exactly the reported symptom.

**This is the headline fix.** Everything CipSoft draws with `rand()`/`random()` must come from the
single `sim_glibc_rand` stream, in the same call order. `ai_rng` should be retired from the 772
path (or *be* the glibc generator). Once unified, the `melee_realign` hack can be deleted.

## Finding 9. `SearchFlightField` shuffle: wrong algorithm and wrong stream  — HIGH

`search_flight_field` (`monster_distance_step.rs`) reproduces the C++ 9-direction priority list
correctly (axial-preferred → 4 cardinals → 4 diagonals) but shuffles with the rand crate:

```rust
dirs[1..5].shuffle(rng);   // rng == ai_rng
dirs[5..9].shuffle(rng);
```

CipSoft `RandomShuffle` (`common.hh:206`) is a **forward** Fisher-Yates over glibc
`random(Min,Max)`:

```cpp
for (int Min = 0; Min < Size-1; Min++) {
    int Swap = random(Min, Size-1);            // glibc, inclusive
    if (Swap != Min) std::swap(Buf[Min], Buf[Swap]);
}
```

The rand crate's `SliceRandom::shuffle` is a **backward** Fisher-Yates with different draw count
and order, and here it runs on `ai_rng`. So flee direction is wrong on both axes (algorithm +
stream). Fix: port `RandomShuffle` exactly (forward, `parity_random(min,max)`), shuffle the
`Dir[1..4]` and `Dir[5..8]` sub-slices with it, with trace sites.

## Finding 10. Roam uses `ai_rng`, not glibc `rand()%4`  — HIGH

`monster_idle_roam_step` is structurally faithful — `ROAM_DIRS = [West, East, North, South]`
matches the C++ `switch(rand()%4)` mapping (0=W,1=E,2=N,3=S), 10 attempts, `MovePossible` gate —
but it draws `ai_rng.gen_range(0..4)` instead of `parity_rand_mod(4)`. Wrong stream → roam paths
diverge and the glibc stream isn't advanced for the (up to 10) draws C++ makes. Fix: use
`parity_rand_mod(4)` per attempt.

## Finding 11. Push/kick should use a deterministic order with NO RNG  — refines Finding 6

`TMonster::KickCreature` (`crnonpl.cc:3036`) shoves the blocker using a **fixed** offset order —
`OffsetX={0,0,-1,1}, OffsetY={-1,1,0,0}` = **North, South, West, East** — skipping the kicker's own
tile, requiring the blocker's `MovePossible(Execute=true)` and `!AVOID`, and **killing** the
blocker if no offset works (full-HP damage attributed to the kicker → kill credit/loot/exp, then
`Kill()`). There is **no random draw at all.**

`monster_push.rs::monster_push_creature` does `dirs.shuffle(&mut rand::thread_rng())` over
`[North, West, East, South]`. So it's wrong three ways: (1) random where C++ is deterministic,
(2) non-glibc/non-deterministic stream, (3) different base order. Fix: drop the RNG entirely and
try N, S, W, E in that fixed order; on exhaustion kill the boxed-in blocker with full-HP damage
**attributed to the kicker** so kill credit / corpse / loot / experience route to it (`combat::execute`
into the victim's `damage_map` + `apply_creature_death`), not a bare `remove_creature`.

## Finding 12. Monster item-pushing (`KickBoxes`) is not implemented  — MEDIUM

`TMonster::KickBoxes` (`crnonpl.cc`) shoves a blocking `UNPASS` box/item aside using the same
deterministic N,S,W,E order (requires `BANK && !UNPASS` destination), deleting it if it can't move.
It is invoked from `MovePossible(Execute=true)` when the mover has `CanKickBoxes()` (race flag or
inherited from a monster master).

`monster_push.rs` has `can_push_items` but the branch is a no-op stub
(`// TFS Monster::pushItems — deferred; item cylinder move path not wired here yet`). So
box-kicking monsters (e.g. many `canpushitems="1"` types) bump on tiles a 772 monster would clear.
This contributes to the "stuck/clunky" feel on cluttered tiles. Note `CanKickBoxes` also inherits
from a monster master — wire that too.

## Finding 13. `CreatureMoveStimulus` sleep-wake — verify the gate, looks close

`TMonster::CreatureMoveStimulus` (`crnonpl.cc:2945`): self-move while SLEEPING → IDLE + `ToDoYield`;
another creature moving wakes a sleeper only if it's a **player or player-controlled monster**
(not NPC, not a wild monster), then `IDLE + ToDoYield`; finally calls the base
`TCreature::CreatureMoveStimulus` (target/follow tracking). Rust
`monster_sleep_wake_on_creature_move` matches the wake gate. Confirm the base-class tail (follow/
target re-evaluation on a tracked creature's move) is fully covered by `monster_events.rs`
`onCreatureMove`, not just the sleep transition.

## Revised root-cause picture

The reported symptoms map cleanly onto the findings:

- **"don't react the same" (run-to-run / vs decompile)** → Finding 8 (three RNG streams) + 6/11
  (`thread_rng` push) + 9/10 (flee/roam on `ai_rng`).
- **"bump into each other"** → Findings 5/11 (wrong push model + non-deterministic, wrong-order
  kick) + 12 (no box kicking) + the planning/`MovePossible` gating.
- **"stop for a second"** → Finding 7 (`EXHAUSTED`→`ToDoWait(1000)` not reproduced for the right
  trigger; common contention degrades to clear-queue+replan instead of kick-and-retry).
- **subtle long-run drift even when a fight "passes"** → Finding 8 (glibc stream desync after any
  `ai_rng`/`thread_rng` draw).

## Recommended fix order (supersedes earlier passes)

1. **Unify RNG (Finding 8).** Route all 772 AI/combat draws through `sim_glibc_rand`; retire
   `ai_rng` and `thread_rng` from the 772 path; delete the `melee_realign` resync hack. Everything
   below depends on this for true parity.
2. **Port `RandomShuffle` exactly** and use it for `SearchFlightField` (Finding 9); switch roam to
   `parity_rand_mod(4)` (Finding 10).
3. **Rewrite push/kick from `crnonpl.cc:2141`/`3036`** — deterministic N,S,W,E, gated on
   ATTACKING/PANIC + Target + `KickCreatures`, kick-and-retry-same-tile, `EXHAUSTED`→`Wait(1000)`
   (Findings 5, 6/11, 7); implement `KickBoxes` (Finding 12).
4. **Casting `break`** — remove it so multi-spell monsters cast all gated spells per idle (Finding 2).
   (Finding 1 distance band → **no change**; keep `DistanceKeep::PerType` — see Phase 4.)
5. Emit monster `Talk` (Finding 3); fix the `step_beat_ms` authority (latent); verify
   `CreatureMoveStimulus` tail (Finding 13).

## Areas examined and judged faithful (no action)

Idle phase ordering; drain-based idle trigger; beat/loop cadence (`Beat=200`, `MoveCreatures`);
reverse `TShortway` viewport (10) and node cap; melee chase budget (3); combat scheduling
(`DelayAttack` 200/2000, defense 2000 ms gate); `CanToDoAttack` second chase path (present);
`SearchFlightField` direction *priority* (correct; only the shuffle is wrong); dance choice (uses
the glibc stream correctly). The TFS-style `get_distance_step`/`get_random_step`/`get_dance_step`
in `monster_distance_step.rs` are the **1098** path and are not used by 772 (which has its own
`monster_idle_*` steps) — fine, but note the file mixes both eras.

## Confidence

After three passes the gaps cluster in two roots: **(a) RNG architecture** (Finding 8, with 6/9/10/11
as instances) and **(b) the push/collision layer ported from 1098** (Findings 5/7/11/12). The
decision tree itself (idle ordering, targeting, casting structure, chase branches) is faithful
apart from the distance-band constant (1) and the casting `break` (2). I'm confident these cover
the movement/reactivity "feel" defects; the remaining open item is exact **combat damage-number**
parity (`ProbeValue`/`ComputeDamage`/armor/poison), which can't match until Finding 8 is resolved
and should be re-validated against the oracle afterward.


---

# Pass 4 — combat-formula verification, line-of-sight, and an RNG-scope correction

This pass verified the combat damage math, the line-of-sight primitive, and the exact scope of the
glibc parity stream. It **corrects an overstatement in Pass 3** and adds three findings, one of
which (LOS) has real behavioral impact for distance monsters.

## Correction to Pass 3 Finding 8 — melee damage IS glibc-parity in sim mode

Pass 3 said combat damage "uses `ai_rng`." That was too broad. `combat/math.rs` actually switches
streams:

```rust
// probe_value (attack/defense roll) and armor_reduction:
if sim_glibc_rng_enabled() { sim_probe_random_factor() / sim_rand_mod(half) }  // glibc
else                       { ai_rng.gen_range(..) }                            // live fallback
```

So in **sim mode** (`TFS_SIM_SEED` set) melee attack, defense, and armor rolls draw from the glibc
stream and are parity-correct. `sim_probe_random_factor()` even reproduces the C++
`((rand()%100)+(rand()%100))/2` two-draw average exactly, and the probe formula
`Max = attack*(skill*5+50)`, `Result = factor*Max/10000` matches `TSkillProbe::ProbeValue`
(`crskill.cc:535`) and `GetAttackDamage`/`GetArmorStrength`. **These are faithful.** The Pass-3
"combat damage" concern was wrong; the real residual `ai_rng` leaks in sim mode are narrower
(roam, flee shuffle, and spell-damage variation — see below).

## Finding 14. Monster spell-damage variation uses `ai_rng`, not glibc — sim-parity leak  — MEDIUM

`monster_idle_apply_spell_impact` (Damage arm) computes:

```rust
let scaled = spell_damage(&profile, hooks, 0, 0, max_dmg, false, false); // == 0 for monsters
let dmg = if scaled > 0 { scaled } else { uniform_random(rng, min_dmg, max_dmg).max(0) }; // ai_rng
```

CipSoft `ComputeDamage` for a **monster** actor (`magic.cc:776`) is just
`Damage + random(-Variation, Variation)` — no level/magic scaling (that branch is `Type==PLAYER`
only). So:

- The **distribution is correct** (`uniform[base-var, base+var]`), so live behaviour is fine.
- But the **stream is `ai_rng`**, so in sim mode it doesn't match the glibc draw the decompile
  makes there, and it fails to advance the glibc stream → desync for the rest of that idle.
- Minor smell: routing monster damage through `spell_damage(level=0, magicLevel=0)` and relying on
  it returning 0 is fragile; the monster path should call the variation roll directly via the
  parity stream (`parity_random(-var, var)` analog), mirroring `ComputeDamage`.

Fix with Finding 8: route the variation roll through `sim_glibc_rand` like the condition arm
already does (`parity_random(min_c, max_c)`).

## Finding 15. The entire glibc parity is harness-only and process-global  — INFO / architectural

`enable_sim_glibc_rng()` is called **only** from `init_sim_rng_from_env` when `TFS_SIM_SEED` is in
the environment. `sim_glibc_rng_enabled()` is a single process-global `AtomicBool`, and the
generator is the process-global C library `libc::rand()` / `libc::srand()`.

Implications:

- **Parity is validated only in the headless harness.** On a live 772 server (`TFS_SIM_SEED`
  unset) every `parity_*` / `probe_value` / `armor_reduction` call falls back to `thread_rng` /
  `ai_rng`. That's acceptable for live play (distributions are right) but means "matches the
  decompile" only holds for the harness, not the shipped binary. Worth stating explicitly so the
  parity claim isn't over-read.
- **`libc::rand` is process-global**, not per-`GameWorld` and not thread-isolated. Fine under the
  single-game-thread rule for one world, but multiple worlds or parallel tests touching the glibc
  stream would interfere; parity tests must run serially. A per-world glibc-equivalent generator
  (own state, not `libc`) would be safer and would also let live 772 use the same code path.
- This reframes Pass 3's Finding 8: the stream fragmentation matters most for **sim-vs-decompile
  parity** (roam/flee/spell-var desync). The **live "feel" defects are the structural ones**
  (push model 5/6/11/12, bump handling 7, distance band 1, casting 2) — those change observable
  behaviour regardless of RNG backend.

## Finding 16. Line-of-sight is TFS 1098 `isSightClear` (Bresenham), not 772 `ThrowPossible`  — HIGH

`map/los.rs` implements `Map::is_sight_clear` via a standard **Bresenham** line, headed
`C++ reference: map.cpp Map::isSightClear, canThrowObjectTo` — the **1098** algorithm.

CipSoft 772 sight is `ThrowPossible` (`info.cc:1154`), which is structurally different:

- **Major-axis linear interpolation**: `CurX = (OrigX*(MaxT-T) + DestX*T)/MaxT` sampled over
  `T = StartT..=MaxT` (`MaxT = max(|dx|,|dy|)`) — a different tile set than Bresenham on diagonals.
- Checks the **`UNTHROW`** flag specifically (Rust checks `blocks_sight`, whose flag mapping must
  be confirmed to equal `UNTHROW`, not `UNPASS`).
- **Multi-floor**: iterates `MinZ..=DestZ` with `Power` and `BANK` floor-stepping; Rust returns
  `false` for any `from.z != to.z`.
- **Hook special case**: `StartT = 0` when throwing west/north through `HOOKEAST`/`HOOKSOUTH` — no
  equivalent in Rust.

Impact is real, not cosmetic: `monster_throw_possible` (`monster_targets.rs:481`) gates on
`is_sight_clear`, and that gate **decides the distance-vs-melee branch** in idle
(`monster_idle_uses_dist_branch`) and the ranged-attack opportunity in `monster_do_attacking`.
A different LOS tile set can flip a distance monster between kiting and closing in certain
geometries, change when it can shoot, and change target-loss timing — all observable "doesn't
react the same" behaviour for archers/casters. Fix: implement a 772 `ThrowPossible` (major-axis
interpolation + `UNTHROW` + floor-step) and route 772 sight/throw through it; keep Bresenham
`isSightClear` for 1098.

## Pass 4 verdict additions

- **Faithful (verified this pass):** melee attack/defense probe (`((rand%100+rand%100)/2)*Max/10000`,
  `Max=attack*(skill*5+50)`), randomized armor `(A/div)+rand%(A/div)`, fight-mode integer modifiers
  (+20/−40 atk, −40/+80 def), `DelayAttack` 200/2000, defense 2000 ms gate, exp polynomial/distribution,
  condition ticks, dance direction order (`DANCE_DIR_ORDER` W,E,N,S,hold via the glibc stream).
- **New gaps:** spell-damage variation stream (14), LOS algorithm (16, behavioural).
- **Clarified:** glibc parity is harness-only and process-global (15); the earlier "combat uses
  ai_rng" claim is corrected — only roam (10), flee shuffle (9), and spell-var (14) leak in sim.

## Consolidated gap list (all passes)

> **Implementation status (authoritative per-phase headers above):** Findings 1 (reclassified, no
> change), 2, 5, 6/11, 7, 9, 10, 12, 14, 16/16b, 17/17b, 19 are **implemented** (Phases 1–5). Remaining:
> Finding 8/15 RNG retirement + `melee_realign` deletion + per-world generator (needs oracle
> re-baseline), and Phase 6 lifecycle polish (3, 18, 20). The table below is the original snapshot.

| # | Gap | Severity | Live feel | Sim parity |
|---|-----|----------|-----------|------------|
| 1 | Distance band per-type vs hardcoded 4 — reclassified: no change (data uniform 4) | N/A | – | – |
| 2 | Casting loop `break` (one spell/idle) | HIGH | ✔ | ✔ |
| 3 | Monster `Talk` not emitted | LOW | ✔ (cosmetic) | – |
| 5 | Push ported from 1098, wrong gating | HIGH | ✔ | ✔ |
| 6/11 | Push dir random+`thread_rng`; 772 is deterministic N,S,W,E | HIGH | ✔ | ✔ |
| 7 | Blocked-step: no kick-and-retry; `EXHAUSTED`→`Wait(1000)` trigger | MED | ✔ | ✔ |
| 8 | RNG fragmentation (glibc / `ai_rng` / `thread_rng`) | HIGH | partial | ✔ |
| 9 | `SearchFlightField` shuffle: wrong algo + stream | HIGH | low (dist ok) | ✔ |
| 10 | Roam uses `ai_rng` not `parity_rand_mod(4)` | MED | low (dist ok) | ✔ |
| 12 | `KickBoxes` (item push) unimplemented | MED | ✔ | ✔ |
| 13 | `CreatureMoveStimulus` base tail — verify | LOW | ? | ? |
| 14 | Spell-damage variation uses `ai_rng` | MED | low (dist ok) | ✔ |
| 15 | glibc parity harness-only + process-global | INFO | – | scope |
| 16 | LOS is 1098 Bresenham, not 772 `ThrowPossible` | HIGH | ✔ (archers) | ✔ |
| — | `step_beat_ms` 50 vs `Beat` 200 quantizer | LOW | masked | harness only |

## Confidence after Pass 4

The gaps now fall into four buckets: **(a) decision-tree constants** (1, 2), **(b) the
push/collision layer ported from 1098** (5, 6/11, 7, 12), **(c) RNG stream fidelity for
sim parity** (8, 9, 10, 14, 15), and **(d) the LOS primitive** (16). Combat damage math is
verified faithful. The remaining unverified item is the `CreatureMoveStimulus` base-class tail
(13) and the exact `blocks_sight`↔`UNTHROW` flag mapping (part of 16). I'm confident this set
covers the movement, reactivity, distance-combat, and sim-parity surfaces; a further pass would be
into spawn/Monsterhome lifecycle and summon/convince mechanics, which are adjacent to but not the
reported melee/distance "feel" symptoms.


---

# Pass 5 — spawn/Monsterhome lifecycle, leash bounds, and a LOS flag refinement

This pass audited the spawn/respawn lifecycle, the roam/chase leash, and closed the open
`blocks_sight`↔`UNTHROW` question from Finding 16. The spawn *placement* search is a faithful 772
port; the *respawn timing* and the *chase leash* are not.

## Faithful (verified): 772 spawn tile placement

`spawn_placement.rs` is a genuine port of `ProcessMonsterhomes` / `SearchSpawnField`:

- `shrink_spawn_radius_near_players` matches `crnonpl.cc:1427-1455`: search window `(R+9, R+7)`,
  per-player shrink `radius = max(distX-9, distY-7)`, `R` clamped to `[*,10]`.
- `classic772_signed_search_distance` matches the `ActMonsters==0 → ≤1` first-spawn clamp and the
  `-R` extended search for later spawns.
- `search_spawn_field` reproduces the BFS expansion + login/clean tie-break shape of
  `SearchSpawnField`; `search_login_field` the east-first spiral.

Two nits inside it:
- The player-proximity check skips players on a different floor (`pos.z != home.z`); CipSoft uses
  `Player->CanSeeFloor(MH->z)`, which counts players one floor up/down who can see the home floor.
  Edge case — can spawn a monster CipSoft would have blocked.
- The BFS tie-break draws `rand::thread_rng().gen_range(0..100)`; CipSoft uses glibc `random(0,99)`
  (`SearchSpawnField`). Same `thread_rng` non-parity/non-determinism as Findings 6/8 — see 19.

## Finding 17. Chase leash applied during active chase — 772 exempts ATTACKING/PANIC  — HIGH

Rust `monster_can_occupy_chase_tile` (the gate for every roam/chase step) unconditionally rejects
tiles failing `is_in_spawn_range(pos, spawn, cfg.despawn_radius, cfg.despawn_z_range)`.

CipSoft `TMonster::MovePossible` (`crnonpl.cc:2148-2167`) applies the radius bound **only when not
attacking**:

```cpp
if (this->State != ATTACKING && this->State != PANIC) {
    if (!MonsterhomeInRange(this->Home, x, y, z)) return false;
    if (max(|x-posx|, |y-posy|) > this->Radius) return false;
}
```

So a 772 monster in ATTACKING/PANIC **follows its target out of the home radius**; the leash is
enforced later by the IdleStimulus despawn check (`!MonsterhomeInRange → StartLogout + SLEEPING`).
The Rust monster instead **refuses the step** at the despawn-radius edge — it gets pinned at an
invisible boundary while the target walks away, rather than chasing out and (eventually)
despawning. How visible this is depends on `despawn_radius`, but the gate is structurally in the
wrong place: it should be skipped for ATTACKING/PANIC chase tiles, with despawn handled by the
out-of-range path (`monster_handle_out_of_spawn_range`, which already exists).

**17b — global radius vs per-home `Radius`.** The leash uses one global
`monster_world_config.despawn_radius`; CipSoft bounds each monster by **its own Monsterhome
`Radius`** (`MonsterhomeInRange` / `MovePossible`). Roam range therefore won't vary per spawn the
way 772 does. The roam bound itself (for non-attacking wandering) is otherwise correct in spirit
(reject out-of-range tiles), just not per-home.

## Finding 18. Respawn timing is the TFS 1098 fixed-`spawntime` model, not 772 `StartMonsterhomeTimer`  — MEDIUM

`spawn.rs`: `respawn_at = now + Duration::from_millis(slot.spawntime_ms)` — a fixed per-slot
interval, headed `C++ Spawn::checkSpawn` (`spawn.cpp`, the **1098** tree).

CipSoft `StartMonsterhomeTimer` (`crnonpl.cc:1296`):

```cpp
int MaxTimer = MH->RegenerationTime;
if (NumPlayers > 800)      MaxTimer = MaxTimer * 2 / 5;
else if (NumPlayers > 200) MaxTimer = MaxTimer * 200 / (NumPlayers/2 + 100);
MH->Timer = random(MaxTimer/2, MaxTimer);     // glibc
```

772 respawn time is **randomized** in `[regen/2, regen]` and **scales down with server
population** (faster respawns when crowded). The Rust model is deterministic, unscaled, and
not randomized. Live-observable: respawn cadence is too regular and doesn't speed up under load.
Fix for 772: model `RegenerationTime` + the player-count scaling + the `random(regen/2, regen)`
draw (through the parity stream).

## Finding 19. Spawn tile tie-break uses `thread_rng` — extends Finding 8  — LOW/MED

`search_spawn_field`'s tie-break (`rng.gen_range(0..100)`) and the implicit placement randomness
use `rand::thread_rng()`, where CipSoft `SearchSpawnField` uses glibc `random(0,99)`. So *where*
within a home a monster spawns is non-deterministic and non-parity. Low live impact (any valid
tile is fine) but breaks sim parity and reproducibility — same class as Findings 6/8.

## Finding 16 refinement — `blocks_sight` flag set is over-broad

`blocks_sight` tests `BLOCK_SOLID | BLOCK_PROJECTILE`. 772 `ThrowPossible` checks **only**
`UNTHROW` (projectile-blocking), not `UNPASS` (solid). A tile that is solid-but-throwable would
block Rust sight but not 772 sight. So Finding 16's fix should also narrow the 772 sight test to
the `UNTHROW`-equivalent flag alone (`BLOCK_PROJECTILE`), independent of `BLOCK_SOLID`.

## Pass 5 additions to the gap list

| # | Gap | Severity | Live feel | Sim parity |
|---|-----|----------|-----------|------------|
| 17 | Chase leash not exempt in ATTACKING/PANIC (monster pinned at radius edge) | HIGH | ✔ | ✔ |
| 17b | Global despawn radius vs per-Monsterhome `Radius` | MED | ✔ | ✔ |
| 18 | Respawn timing fixed (1098) vs 772 `random(regen/2,regen)`+crowd scaling | MED | ✔ | ✔ |
| 19 | Spawn tile tie-break `thread_rng` vs glibc `random(0,99)` | LOW | low | ✔ |
| 16b | `blocks_sight` tests `BLOCK_SOLID\|BLOCK_PROJECTILE`; 772 uses `UNTHROW` only | MED | ✔ (archers) | ✔ |

## Confidence after Pass 5

The audited surface now spans: idle decision tree, targeting, casting, combat scheduling + damage
formulas, walk timing + loop cadence, pathfinding/TShortway, push/kick, blocked-step recovery,
flee/roam/dance, RNG architecture, line-of-sight, spawn placement + respawn + leash. The gaps
group into five clusters:

1. **Decision-tree constants** — distance band 4 (1), casting `break` (2).
2. **Push/collision ported from 1098** — push model/gating (5), deterministic kick order (6/11),
   blocked-step kick-and-retry/`EXHAUSTED` (7), `KickBoxes` (12).
3. **RNG stream fidelity (sim parity)** — fragmentation (8), flee shuffle algo+stream (9), roam
   stream (10), spell-var stream (14), spawn tie-break (19); scope/global-state (15).
4. **Line-of-sight** — 1098 Bresenham vs 772 `ThrowPossible` + flag set (16/16b).
5. **Spawn/leash lifecycle** — chase-leash exemption (17), per-home radius (17b), 772 respawn
   timing (18).

Combat damage math, spawn *placement*, idle ordering, beat cadence, and dance RNG are verified
faithful. Remaining genuinely-unaudited corners are narrow: summon/convince/challenge behaviour
(`ConvinceMonster`/`ChallengeMonster`), NPC AI (`TNPC::IdleStimulus`), and exact field/condition
interactions (fire/poison field avoidance in `MovePossible` `AVOID` handling). None of those are
implicated by the reported melee/distance "feel" symptoms, which are fully explained by clusters
1, 2, 4, and 5 (live) plus 3 (sim parity). I'd consider the AI-movement/combat audit substantively
complete; a Pass 6 would only be worthwhile to cover summons/NPCs if those are in scope.


---

# Pass 6 (final) — summons, convince/challenge, and closing synthesis

Final pass covering the last corners: summon lifecycle, convince/challenge, and confirmation that
the TFS target-change mechanic is correctly disabled on 772.

## Faithful (verified): no TFS `onThinkTarget` on 772

`monster_on_think_target` (the TFS 1098 `changeTargetChance`/`changeTargetSpeed` retarget,
`monster.cpp:923`) early-returns when `beat_driven_loop` is set, and `monster_native_on_think`'s
772 branch never calls it. 772 retargeting runs only from the idle `Strategy[]` pick
(`crnonpl.cc:2468`). Correct — this was a plausible divergence and it's handled. (It does use
`rand::random()` for the 1098 path, but that's outside 772.)

## Finding 20. Summon lifecycle is a stub — master despawn conditions missing  — MEDIUM

`monster_think_summon_stub` implements summon **target inheritance** (follow the master's attack
target, else follow the master), and the `MasterFollow` idle walk branch handles the manhattan
2–3 wait band. But the 772 summon **despawn / re-bind** block at the top of `TMonster::IdleStimulus`
(`crnonpl.cc:2360-2402`) is not reproduced:

- `Master == NULL` → `Kill()` (or `StartLogout` if player-mastered).
- player master with `SummonedCreatures == 0` (master relogged) → despawn.
- non-player master on a different floor → despawn.
- `|Δz| > 1 || |Δx| > 30 || |Δy| > 30` → despawn.
- `Master->Combat.Following ? Target = 0 : Target = Master->Combat.AttackDest`, then
  `Target == 0 || Target == self → Target = Master`.

So Rust summons won't self-despawn when the master logs out, dies, or strays beyond 30 tiles /
a floor, and the "follow vs inherit master's attack target" gating is approximate rather than the
exact `Combat.Following` rule. Live-observable as orphaned/lingering summons and slightly wrong
summon target selection. (Summon **placement** via `SearchSummonField` — `info.cc` — is also worth
checking if summon spells are wired.)

## Finding 21. Convince / Challenge are 772-shaped differently than the 1098 fields present  — LOW

CipSoft (`crnonpl.cc:3177/3196`):

- `ChallengeMonster` → `Monster->Target = Challenger->ID; Rotate(Challenger); ToDoYield();` —
  a one-shot forced target, **no focus duration**.
- `ConvinceMonster` → `Slave->Convince(Master)` — the monster becomes a summon of the convincer.

The Rust carries a 1098-style `challenge_focus_duration` (TFS `Monster::challengeFocusDuration`,
"blocks flee while challenged") but no 772 `ChallengeMonster` (set-target+rotate+yield) and no
`Convince`. If the 772 challenge/convince runes are in scope, they need the 772 semantics, not the
1098 focus-duration model. Low priority (player-rune-driven, niche), and likely routed through the
spell/Lua layer rather than core monster AI — flag for whoever wires those runes.

## Out of audited scope (not implicated by the reported symptoms)

- `TNPC::IdleStimulus` (NPC wander/behaviour-tree) — NPCs, not monsters.
- Field/condition `AVOID` handling inside `MovePossible` (fire/poison/energy field avoidance,
  `NoPoison`/`NoBurning`/`NoEnergy`, PANIC ignores hazards) — partially visible in the
  `MovePossible` read (Pass 2) but not cross-checked against the Rust field-avoidance path.
- `SearchSummonField` summon placement.

These are noted for completeness; none relate to the melee/distance reactivity that started this
audit.

---

# Final synthesis & prioritized roadmap

After six passes the audit is substantively complete for monster movement and combat. The reported
symptoms — melee monsters bumping/stalling, distance monsters kiting wrong, "doesn't react the
same" — are explained by a small number of root causes, almost all of which are **the 772 port
reusing TFS 1098 logic** where the eras genuinely differ.

## Root-cause clusters

1. **Push/collision ported from 1098** (Findings 5, 6/11, 7, 12) — the single biggest *live* melee
   defect. 772 uses `MovePossible` + deterministic-order `KickCreature`/`KickBoxes` gated on
   ATTACKING/PANIC + `KickCreatures`, with kick-and-retry and `EXHAUSTED`→`Wait(1000)`. Rust uses
   the 1098 push-then-kill model with a `thread_rng` shuffle.
2. **Line-of-sight era** (Findings 16/16b) — 1098 Bresenham + over-broad flags vs 772
   `ThrowPossible` interpolation + `UNTHROW`. Drives the distance-vs-melee branch, so it warps
   archer/caster behaviour.
3. **Spawn/leash lifecycle** (Findings 17/17b, 18, 19) — chase leash not exempt during attack
   (monsters pinned at radius edge), global vs per-home radius, 1098 fixed respawn vs 772
   randomized/crowd-scaled.
4. **Decision-tree constants** (Findings 1, 2) — distance band per-type vs hardcoded 4; casting
   `break` (one spell/idle vs all).
5. **RNG stream fidelity** (Findings 8, 9, 10, 14, 19; scope 15) — mostly a *sim-parity* concern
   (roam/flee/spell-var/spawn draw from `ai_rng`/`thread_rng`, not the glibc stream); melee damage
   and dance are already correct in sim mode.

## Suggested fix order (impact-first)

1. **Push/collision rewrite from `crnonpl.cc:2141`/`3036`/`KickBoxes`** — fixes the headline melee
   clunk; deterministic N,S,W,E, correct gating, kick-and-retry, `EXHAUSTED`→`Wait(1000)`.
2. **772 `ThrowPossible` LOS** (+ narrow `blocks_sight` to `UNTHROW`) — fixes distance/caster
   behaviour and the branch selection it feeds.
3. **Chase-leash exemption for ATTACKING/PANIC** + per-home radius — stops the "stuck at the
   leash" pin.
4. **Remove casting `break`** — multi-spell monsters cast all gated spells per idle (Finding 2).
   (Finding 1 distance band needs **no change** — `DistanceKeep::PerType` already matches 772 on the
   shipped uniform-4 data; see Phase 4.)
5. **Unify RNG onto the glibc stream** (retire `ai_rng`/`thread_rng` on the 772 path; port
   `RandomShuffle`; route roam/spell-var/spawn tie-break through it; delete the `melee_realign`
   hack) — restores sim-vs-decompile parity; consider a per-world glibc generator so live 772 uses
   the same path (Finding 15).
6. **772 respawn timing** (`random(regen/2,regen)` + crowd scaling); **summon despawn** lifecycle;
   monster `Talk` emission. Lower-priority polish.

## Verified-faithful (no action)

Idle phase ordering and drain trigger; beat/loop cadence (`Beat=200`, `MoveCreatures`); reverse
`TShortway` viewport/cap; melee chase budget (3); combat scheduling (`DelayAttack` 200/2000,
defense 2000 ms gate) and damage math (`ProbeValue` two-draw average, randomized armor, fight-mode
modifiers, exp curve/distribution, condition ticks); dance direction + RNG; 772 spawn *placement*
(radius-shrink, first-spawn clamp, BFS); `onThinkTarget` correctly disabled on 772.

## Status

Audit complete to the level of "every structural gap in monster movement, combat, targeting,
spawn, and LOS is identified and cited." Genuinely remaining unknowns are narrow and out of the
reported-symptom path (NPC AI, field-`AVOID` cross-check, summon placement). The document is the
reference for the implementation work; each finding cites the exact C++ function/line and the Rust
site to change.


---

# Implementation / Update Plan

Phased plan to close the findings, ordered impact-first. Each phase is independently shippable and
ends at a green gate (`rtk cargo check` + `rtk cargo clippy` + targeted `rtk cargo test`). Capture
any C++-behaviour surprises in `tasks/lessons.md` as you go. Keep all version branching behind the
`MechanicsProfile` / `beat_driven_loop` seams — no `client_version` checks in core.

Convention per phase: **Goal · Findings · C++ ref · Rust sites · Steps · Verify · Risk.**

## Phase 1 — Push / collision rewrite (biggest live melee win)

- **Goal:** melee monsters shove through each other deterministically instead of bumping/stalling.
- **Findings:** 5, 6/11, 7, 12.
- **C++ ref:** `crnonpl.cc:2141` `TMonster::MovePossible`, `:3036` `KickCreature`, `KickBoxes`;
  `crnonpl.cc:2918-2926` + IdleStimulus catch (`EXHAUSTED`).
- **Rust sites:** `monster_push.rs`, `walk/mod.rs` (~1352 blocked-step), `monster_ai.rs`
  (`monster_tshortway_fill_walkable`, `monster_can_occupy_chase_tile`).
- **Steps:**
  1. Re-point `monster_push.rs` headers/logic at the 772 source. Gate kicking on
     `State ∈ {ATTACKING, PANIC}` + `Target != 0` + mover `KickCreatures`.
  2. Replace the random shuffle with the **fixed N, S, W, E** offset order
     (`{0,0,-1,1}/{-1,1,0,0}`), skip the mover's own tile, require blocker `MovePossible(Execute)`
     + `!AVOID`; **kill** the blocker only when all four fail — with full-HP damage attributed to
     the kicker (`combat::execute` → victim `damage_map` → `apply_creature_death` for kill
     credit/corpse/loot/exp), not a bare remove.
  3. Implement `KickBoxes` (same fixed order, `BANK && !UNPASS` dest, delete on failure) and wire
     `CanKickBoxes` (race flag + monster-master inheritance).
  4. Blocked-step: kick-and-retry the **same** tile in-step; route genuine `EXHAUSTED` to
     `Wait(1000)` (`MONSTER_IDLE_WAIT_MS`) instead of clear-queue+replan.
  5. Confirm `MovePossible(Execute=false)` planning treats kickable-creature tiles as
     plannable-through and hard-blocks for non-`KickCreatures` movers.
- **Verify:** new tests — two `canpushcreatures="0"` monsters route around each other; two
  `canpushcreatures="1"` shove in N,S,W,E order; box-blocked tile gets cleared; a boxed-in pushable
  monster is killed with kill credit/loot/exp to the kicker; `EXHAUSTED` waits 1000 ms. No
  `thread_rng`/`ai_rng` in the push path.
- **Risk:** medium — touches the hot step path; keep the kill-on-exhaustion path behind the same
  conditions as C++ to avoid over-killing allies.

## Phase 2 — 772 line-of-sight (`ThrowPossible`)  — IMPLEMENTED

**Status:** done. `Map::throw_possible(orig, dest, power)` (`map/los.rs`) ports `ThrowPossible`
exactly — major-axis interpolation, `UNTHROW`-only flag test (new `tile::flags::UNTHROW` from
`block_projectile()`), multi-floor `MinZ` stepping, and the `HOOKEAST`/`HOOKSOUTH` `StartT=0` origin
case (new hook flags from `is_hangable()` + `is_horizontal/vertical()`). `GameWorld::monster_sight_clear`
dispatches 772 → `throw_possible(.,.,0)` / 1098 → Bresenham `is_sight_clear`, and all monster/combat
sight callers route through it (monster_targets, monster_ai dist-branch + ranged, idle_stimulus spell
+ trace, creature_think). Bresenham `is_sight_clear` is untouched for 1098. Tests in `tests/map_los.rs`.

- **Goal:** correct distance-vs-melee branch + ranged/spell sight for archers/casters.
- **Findings:** 16, 16b.
- **C++ ref:** `info.cc:1154` `ThrowPossible`; `common`/object flags for `UNTHROW`.
- **Rust sites:** `map/los.rs`, `map/mod.rs` (`blocks_sight`), `monster_targets.rs`
  (`monster_throw_possible`), `idle_stimulus.rs` sight checks, `monster_ai.rs` ranged attack.
- **Steps:**
  1. Add `Map::throw_possible(orig, dest, power)` implementing the major-axis interpolation
     (`Cur = (orig*(MaxT-T)+dest*T)/MaxT`), `UNTHROW` check, multi-floor `MinZ` stepping, and the
     `HOOKEAST`/`HOOKSOUTH` `StartT=0` case.
  2. Route 772 monster sight/throw (`monster_throw_possible`, dist-branch gate, ranged attack,
     spell `is_sight_clear` calls) through it via the profile seam; keep Bresenham `is_sight_clear`
     for 1098.
  3. Narrow the 772 sight flag test to the `UNTHROW`-equivalent (`BLOCK_PROJECTILE`) only, not
     `BLOCK_SOLID`.
- **Verify:** golden tests against `ThrowPossible` for axial/diagonal/blocked/hook cases; a
  distance monster's branch selection matches the oracle in a geometry where Bresenham and
  interpolation differ.
- **Risk:** medium — multi-floor + hook edges are fiddly; start same-floor, add floor-stepping next.

## Phase 3 — Chase leash + roam bounds  — IMPLEMENTED

**Status:** done. `monster_can_occupy_chase_tile` and `monster_tshortway_fill_walkable` now skip the
`is_in_spawn_range` leash entirely for ATTACKING/PANIC (Finding 17 — the monster chases out of range
and despawns via the existing `monster_handle_out_of_spawn_range` path). The non-attacking roam leash
uses a per-monster `home_radius` (new field on `Monster`, set from the spawn zone `radius` in
`spawn_monster`) via `monster_roam_leash_radius`, falling back to the global despawn radius when unset
(`home_radius <= 0`) or on 1098 (Finding 17b). Tests in `monster_ai.rs`.

- **Goal:** monsters follow targets out of the home radius (then despawn), not pin at the edge.
- **Findings:** 17, 17b.
- **C++ ref:** `crnonpl.cc:2148-2167` `MovePossible` radius block; `:1515` `MonsterhomeInRange`;
  IdleStimulus despawn (`:2407`).
- **Rust sites:** `monster_ai.rs` (`monster_can_occupy_chase_tile`, `is_in_spawn_range`),
  `creature/monster.rs` (per-monster home radius field), `spawn*`.
- **Steps:**
  1. Skip the `is_in_spawn_range` leash for chase tiles when `State ∈ {ATTACKING, PANIC}`; keep it
     for roam.
  2. Use a per-monster home radius (carried from its Monsterhome/spawn slot) instead of the global
     `despawn_radius`; keep despawn handled by the out-of-range path
     (`monster_handle_out_of_spawn_range`).
  3. Confirm roam still rejects out-of-`Radius` tiles (non-attacking) per `MovePossible`.
- **Verify:** a chasing monster steps beyond its radius following a player and despawns when out of
  `MonsterhomeInRange`; a roaming monster stays within `Radius`.
- **Risk:** low/medium — ensure the despawn path actually fires so monsters don't chase forever.

## Phase 4 — Decision-tree constants

- **Goal:** correct multi-spell casting. (Distance band — Finding 1 — needs **no change**, see below.)
- **Findings:** 1 (reclassified — no change), 2 (actionable).
- **C++ ref:** `crnonpl.cc:2795-2834` (band `4`); `:2521-2667` (cast loop, no `break`).
- **Rust sites:** `idle_stimulus.rs` (`monster_idle_try_casting`).

### Finding 1 — distance band: NO CODE CHANGE (keep `DistanceKeep::PerType`)

Reassessed against the shipped data: **all** 772 distance monsters use `targetdistance="4"` and all
melee use `"1"` (verified: 29× `4`, 128× `1`, no other value in `data/monster/`). Under
`DistanceKeep::PerType` the band sites resolve to the exact C++ literals when the value is 4 — flee
`dist < 4`, chase budget `cheb - 4`, dance `dist == 4` — so per-type is **behaviorally identical** to
the hardcode for this data. There is no active divergence.

Do **not** pin `DistanceKeep::Fixed(4)`:
- `target_distance` does double duty as the **melee-vs-distance discriminator**
  (`monster_idle_uses_dist_branch` = `target_distance > 1 && monster_throw_possible(...)`). Forcing
  `Fixed(4)` makes `monster_effective_target_distance` return 4 for every monster, collapsing the
  discriminator to "has a ranged spell?" — a `targetdistance="1"` melee monster that also carries a
  ranged spell would flip into a kiter. `PerType` keeps it melee. So `Fixed(4)` adds risk with zero
  upside on uniform-4 data.

The only residual exposure is **data drift**: a distance monster authored with `targetdistance != 4`
(e.g. a pasted 1098 `monster.xml`) would kite/dance/flee at a non-772 ring. CipSoft 772 cannot express
a per-type keep distance at all (the `.mon` files have no such field — the band is a global hardcode),
so the band knob is a 1098-ism that merely coincides with 772 at 4.

- **Optional guard (not required):** a warn-only load-time validation that flags any 772
  distance-fighter (`targetdistance > 1`) whose value `!= 4`, so bad content can't silently produce
  un-772 kiting. Behavior-preserving; implement only if desired.

### Finding 2 — remove the casting `break` (IMPLEMENTED)

`crnonpl.cc` CASTING (`~2521-2667`) loops `for SpellNr = 1..=Spells`, each gated by its own
`rand() % Delay`, with **no `break`** — a multi-spell 772 monster can fire several impacts in one idle
and **draws a delay roll for every spell every idle**. Rust `monster_idle_try_casting` broke after
the first successful cast, which (a) capped multi-spell monsters at one cast/idle and (b) skipped the
remaining offensive spells' delay rolls, desyncing the glibc stream.

**Status:** done — the `if cast_any { break; }` is removed (`idle_stimulus.rs`); every spell whose
delay/flee/range/sight gates pass is evaluated and cast in the same idle. Single-spell monsters
(most fixtures) are unaffected.

## Phase 5 — RNG unification (sim parity)

> **Status (partial — Findings 9/10/14/19 done; 8/15 remaining):** the concrete per-draw routing is
> implemented — `RandomShuffle` ported exactly as `sim_glibc_rand::parity_random_shuffle` (forward
> Fisher-Yates over `parity_random`) and used for both `SearchFlightField` sub-slices (9); roam now
> draws `parity_rand_mod(4)` (10); monster spell-damage/heal/speed variation draws `parity_random`
> on 772 (14); spawn tile tie-break draws `parity_random(0,99)` (19). These all flow through the one
> `sim_glibc_rand` stream in sim mode (and `thread_rng` live), removing the residual `ai_rng`/`thread_rng`
> leaks the audit flagged.
>
> **Remaining (Finding 8/15, deferred):** fully retiring `ai_rng` from the 772 path, deleting the
> `TFS_SIM_MELEE_REALIGN` hack, and the per-`GameWorld` glibc generator. These require **re-baselining
> the C++-oracle golden RNG traces** (the harness battery `run_sim_battery.py` compares against
> captured `.jsonl` oracle logs), which needs the CipSoft harness — out of band for this change. The
> melee-realign hack is inert outside seeded harness runs (gated on `sim_glibc_rng_enabled()`), so it
> stays in place until the oracle re-baseline confirms the streams align without it.

- **Goal:** all 772 AI/combat draws come from one glibc-equivalent stream, in C++ order.
- **Findings:** 8, 9, 10, 14, 19; 15 (architecture).
- **C++ ref:** `common.hh:206` `RandomShuffle`; `info.cc:1030` `SearchFlightField`;
  `magic.cc:776` `ComputeDamage`; `info.cc` `SearchSpawnField`.
- **Rust sites:** `sim_glibc_rand.rs`, `monster_ai.rs` (roam/flee), `monster_distance_step.rs`
  (`search_flight_field`), `idle_stimulus.rs` (spell-var), `spawn_placement.rs` (tie-break),
  `game_world.rs` (`ai_rng`).
- **Steps:**
  1. Port `RandomShuffle` exactly (forward Fisher-Yates, `parity_random(min,max)`); use it for the
     two `SearchFlightField` sub-slices (9).
  2. Switch roam to `parity_rand_mod(4)` (10), spell-damage variation to a glibc `random(-var,var)`
     analog (14), spawn tie-break to glibc `random(0,99)` (19).
  3. Retire `ai_rng`/`thread_rng` from the 772 path; delete the `TFS_SIM_MELEE_REALIGN` hack once
     draws align.
  4. (Stretch, Finding 15) Replace process-global `libc::rand` with a per-`GameWorld`
     glibc-equivalent generator so live 772 uses the same code path and parallel tests don't
     interfere.
- **Verify:** RNG trace (`TFS_SIM_RNG_TRACE`) call counts/values match the C++ oracle across a flee
  + roam + multi-hit scenario; harness battery green without realign.
- **Risk:** medium — re-baselining golden logs; do it after Phases 1–4 so the structure is final.

## Phase 6 — Lifecycle polish

> **Status:** DONE. Respawn, summon lifecycle, and monster talk implemented + tested.

- **Goal:** respawn cadence, summon cleanup, monster talk.
- **Findings:** 18, 20, 3.
- **C++ ref:** `crnonpl.cc:1296` `StartMonsterhomeTimer`; `:2360-2402` summon despawn; `:2393` Talk.
- **Rust sites:** `spawn.rs`/`spawn_lifecycle.rs`, `monster_ai.rs` (`monster_think_summon_stub`),
  `idle_stimulus.rs` (`monster_idle_try_talk`, `monster_idle_summon_lifecycle_772`).
- **Implementation:**
  1. **772 respawn** (`RespawnModel::Monsterhome772`): `compute_respawn_delay_ms` in
     `spawn_lifecycle.rs` — `random(regen/2, regen)` via parity stream, scaled by online player
     count (`>800 → *2/5`, `>200 → *200/(n/2+100)`). 1098 stays `Fixed`. Gated by
     `MechanicsProfile::respawn_model` + `data/formulas/{772,1098}.lua`.
  2. **Summon despawn/re-bind** (`monster_idle_summon_lifecycle_772`): master gone → despawn;
     non-player master on different floor → despawn; `|Δz|>1 || |Δx|>30 || |Δy|>30` → despawn;
     re-bind `Combat.Following ? Target=0 : Target=AttackDest`, fallback `Target=Master`. Runs
     BEFORE the sleeping/idle check (matches C++ `IdleStimulus` ordering).
  3. **Monster Talk** (`monster_idle_try_talk`): emits `0xAA` via era-aware
     `Codec::encode_creature_say` (772 omits `level` field). `#y `/`#Y ` prefix →
     `TALKTYPE_MONSTER_YELL=0x10`, else `TALKTYPE_MONSTER_SAY=0x11`. Talk texts from
     `<voices><voice sentence="…"/></voices>` in monster XML.
- **Verify:** 10 new tests (respawn band/scaling/fixed, summon despawn/rebind, talk emit/no-emit);
  399 existing tests still pass.
- **Risk:** low.

## Phase 7 — PANIC→ATTACKING promotion + IdleStimulus catch-all (Findings 30, 31)

> **Status:** not started. Low-impact polish; independent of Phases 1–6. Can run in parallel
> with GL Phase 7–8.

- **Goal:** PANIC→ATTACKING promotion fires unconditionally in melee band (not just on
  successful dance step); IdleStimulus error handling is comprehensive (not point-specific).
- **Findings:** 30, 31.
- **C++ ref:** `crnonpl.cc:2832–2834` (PANIC promotion, unconditional in melee band);
  `crnonpl.cc:2890–2898` (catch block — `Target=0; ToDoClear(); if EXHAUSTED Wait(1000)`).
- **Rust sites:** `monster_ai.rs:1349–1381` (`monster_idle_dance_step` — promotion after
  early-return guard), `idle_stimulus.rs:152–164` (`monster_exhausted_wait_772` — called
  from `monster_push.rs:589` and `walk/mod.rs:1271` only).
- **Steps:**
  1. **Finding 30:** move the `band == 1` PANIC→ATTACKING promotion outside the
     `monster_idle_dance_step` early-return guard (line 1353), so it fires even when the
     dance step is blocked. Alternatively, have the caller
     (`monster_idle_execute_walk_branch` at `idle_stimulus.rs:1772`) perform the promotion
     when `MeleeDance` branch returns `Hold` (dance step failed at melee range).
  2. **Finding 31 (structural, optional):** make `monster_idle_prepare_and_enqueue_go` return
     a `Result` and handle all errors at the call site with `monster_exhausted_wait_772`,
     matching the C++ catch-all semantics. This is not urgent given current coverage (push +
     walk failures are handled), but prevents future divergence if new failure paths are
     added without the handler.
- **Verify:** unit test — PANIC monster at melee range with all dance steps blocked promotes
  to ATTACKING. Existing EXHAUSTED tests still pass.
- **Risk:** low — Finding 30 is a 3-line move; Finding 31 is structural and optional.

## Cross-cutting verification

- After each phase: `rtk cargo check` → `rtk cargo clippy` → targeted `rtk cargo test`, then the
  772 chase/kite harness battery (`chase_kite_sim` + `tests/`), comparing against the C++ oracle
  logs. Phase 5 will require re-baselining the golden RNG traces — do it last.
- Keep `tasks/todo.md` as the live task list and `tasks/lessons.md` for any C++ behaviour that
  differed from this audit's reading.

## Out of plan (revisit only if in scope)

`TNPC::IdleStimulus` (NPC AI), `MovePossible` field-`AVOID` cross-check (fire/poison/energy field

---

# Implementation Status Summary (Jun 2026 re-verification)

## Phase Status

| Phase | Status |
|-------|--------|
| 1 | **PARTIAL** — push/collision rewrite (kick-and-retry loop AI#23 still broken) |
| 2 | **DONE** — 772 line-of-sight (`ThrowPossible`) |
| 3 | **DONE** — chase leash + roam bounds |
| 4 | **DONE** — decision-tree constants (no change needed) |
| 5 | **PARTIAL** — RNG unification (drunk walk + `ai_rng` retirement remaining) |
| 6 | **PLANNED** — lifecycle polish (respawn, summon, talk) |
| 7 | **PLANNED** — PANIC→ATTACKING promotion + IdleStimulus catch-all (Findings 30, 31) |

## Critical Findings — RESOLVED

| Finding | Issue | Status | Notes |
|---------|-------|--------|-------|
| 2 | Casting loop stops after first spell | **FIXED** | Now evaluates every spell (`idle_stimulus.rs:637-696`) |
| 5 | Push logic ported from 1098 era | **FIXED** | 772-specific logic with deterministic N,S,W,E order in `monster_push.rs` |
| 6/11 | Push direction uses `thread_rng` | **FIXED** | Uses deterministic `KICK_DIRS_772` constant |
| 7 | Blocked-step recovery doesn't match EXHAUSTED contract | **FIXED** | `monster_exhausted_wait_772` implemented |
| 9 | SearchFlightField shuffle wrong algorithm/stream | **FIXED** | Uses `parity_random_shuffle` with glibc parity stream |
| 10 | Roam uses `ai_rng` instead of glibc | **FIXED** | Uses `parity_rand_mod(4)` |
| 12 | `KickBoxes` not implemented | **FIXED** | `monster_kick_boxes_772` implemented |
| 14 | Spell-damage variation uses `ai_rng` | **FIXED** | Uses `parity_random` for 772 |
| 16 | LOS is 1098 Bresenham, not 772 `ThrowPossible` | **FIXED** | `monster_targets.rs:433` dispatches to `map.throw_possible` on 772; `los.rs:87-152` implements major-axis interpolation + UNTHROW + multi-floor |
| 17/17b | Chase leash not exempt in ATTACKING/PANIC; global vs per-home radius | **FIXED** | `monster_ai.rs:2695` skips leash for ATTACKING/PANIC; per-monster `home_radius` from spawn zone (`spawn_lifecycle.rs:311`) |

## Medium/Low Findings — PARTIALLY RESOLVED

| Finding | Issue | Status | Notes |
|---------|-------|--------|-------|
| 8 | RNG stream fragmentation | **PARTIAL** | `parity_rng` on `GameWorld` for 772 idle/casting; but `thread_rng` remains in drunk walk, `creature_think.rs`, `monster_push.rs:445`, `monster_ai.rs` distance/dance step, `spawn_lifecycle.rs` shuffles, `spawn.rs:301` |
| 3 | Monster Talk is RNG-only, no packet | **PARTIAL** | RNG draw implemented, packet emission may still be missing |
| 14 (GL) | `Other`/`cron` subsystems | **PARTIAL** | Idle-timeout + ambiente done; raids/autosave missing |

## Remaining Issues — the "smoothness" cluster

| Finding | Issue | Severity | Notes |
|---------|-------|----------|-------|
| **GL#7** | **Execute one action per pop vs atomic zero-delay drain** | **HIGH** | **Primary smoothness defect.** `idle_stimulus.rs:2148` `run_monster_todo_execute` pops one action then returns/re-arms. C++ `Execute` (`cract.cc:783`) is `while(true)` over consecutive `CalculateDelay==0` entries. Rotate+Attack = 2 beats (400 ms) vs C++ 1 beat (200 ms). |
| **22** | **Rotate in ATTACKING/PANIC blocked by `walk_timer_idle` gate** | **HIGH** | `monster_ai.rs:2191` — `walk_timer_idle` returns `next_wakeup.is_none()` on 772; after a Go step the monster always has a wakeup, so `monster_update_look_direction` returns early. C++ `Rotate(Target)` at `crnonpl.cc:2872` runs **unconditionally** in ATTACKING/PANIC before `ToDoAttack`. Compounds GL#7: even if Execute were atomic, the Rotate wouldn't fire. |
| **23** | **Missing retry loop in `MovePossible` kick logic** | **HIGH** | `monster_push.rs:96-179` kicks once and returns. C++ `MovePossible` (`crnonpl.cc:2185-2288`) has `for Attempt in 0..100` — after each kick it breaks the inner loop and **retries the outer loop** to re-check the tile. Single-attempt kick → premature `EXHAUSTED` → `Wait(1000)` stall on ally contention. This is the "bumping into each other, stopping for a second" symptom. |
| **24** | **Z-level change clears follow/attack target** | **HIGH** | `monster_events.rs:190-198` clears both targets when `new_pos.z != old_pos.z`, not era-gated. C++ `CreatureMoveStimulus` (`crmain.cc:920`, `crnonpl.cc:2943`) does **not** clear targets on Z-change — only handles combat re-arm for close-chase. Ramp drops cause target loss → re-acquire delay. |
| **GL#22** | **Drunk-walk stagger is 1098 algo on `thread_rng`** | **HIGH** | `walk/mod.rs:116-131` — 1098 `r/4 > d` curve on `thread_rng()` instead of 772 `rand()%max(7-level,1)` on glibc stream. Wrong algo + wrong RNG + wrong source (`ConditionDrunk` vs `Skills[SKILL_DRUNKEN]` timer-skill). Desyncs every subsequent `parity_*` draw. |
| **GL#24** | **Move-stimulus fan-out uses SlotMap-key sort** | **MEDIUM** | `monster_events.rs:112` — `ids.sort_by_key(|id| id.data().as_ffi())` (creation order) vs C++ creature-list order (spatial sector walk). Affects which monster reacts first to a move event. |
| **25** | **Lose-target missing `IsHouse` + invisibility checks** | **MEDIUM** | `idle_stimulus.rs:365-399` — missing `IsHouse` (`crnonpl.cc:2427`) and `IsInvisible && !SeeInvisible` (`crnonpl.cc:2429`). Monsters chase into houses / after invisible targets. |
| **26** | **`CreatureMoveStimulus` extra `has_go` check** | **MEDIUM** | `monster_events.rs:446` — `!has_attack \|\| has_go` checks entire queue; C++ checks head only (`ToDoList.at(ActToDo)->Code == TDAttack`). Blocks combat re-arm when `[Attack, Go]` queued. |
| **27** | **Waypoint cost source mismatch** | **LOW** | `walk/walk_timing.rs:211` — uses `ground_speed` from items_db vs C++ `WAYPOINTS` tile attribute (`cract.cc:1521`). Latent if data matches. |
| **28** | **Poison field extension missing** | **LOW** | `process_skills.rs:74-86` — doesn't extend poison duration when creature stands on poison field (`crskill.cc:1043` `Cycle += 1`). |
| **30** | **PANIC→ATTACKING promotion only on successful dance step** | **LOW** | `monster_ai.rs:1353` — early-returns before promotion at 1374 when dance step blocked. C++ `crnonpl.cc:2832` promotes unconditionally in melee band. Narrow: cornered PANIC monsters stay PANIC. |
| **31** | **`IdleStimulus` catch block not comprehensive** | **LOW** | `idle_stimulus.rs:152` — `monster_exhausted_wait_772` covers push/walk failures but isn't a catch-all like C++ try/catch (`crnonpl.cc:2890`). Risk: future failure paths may miss cleanup. |
| **Structural** | **`CreatureAction` enum missing Rotate/Talk/Use** | **HIGH** | `creature_todo.rs:65-72` — only Go/Wait/Attack. Blocks atomic Execute (GL#7) fix and forces out-of-band Rotate workaround (AI#22). Root cause, not a symptom. |
| 18 | Respawn timing fixed (1098) vs 772 random + crowd scaling | MED | `spawn.rs:237` — `now + spawntime_ms` instead of `random(regen/2, regen)` with player-count scaling (`crnonpl.cc:1296`) |
| 19 | Spawn tile tie-break uses `thread_rng` | LOW/MED | `spawn_placement.rs` — should use `parity_random(0,99)` (partially done per GL audit; verify) |
| 20 | Summon lifecycle is a stub | MED | `monster_ai.rs:2033` — missing master despawn/re-bind/distance checks |
| 21 | Convince/Challenge are 772-shaped differently | LOW | Has 1098-style `challenge_focus_duration` |

## Smoothness Root Cause Analysis

**Symptom:** "the decompile monster AI is much smoother" — melee monsters feel clunky,
turn slowly, bump and stall, don't react the same way twice.

**Root cause chain (ranked by impact):**

### 1. Atomic `Execute` loop not implemented (GL#7) — THE primary defect

C++ `TCreature::Execute` (`cract.cc:783–890`) is a `while(true)` loop. Each iteration:
1. Check `LockToDo / IsDead / NextWakeup > ServerMilliseconds` → break.
2. Check `NrToDo <= ActToDo` (queue exhausted) → `IdleStimulus` → break.
3. `CalculateDelay()` for the head entry → if `Delay > 0`, re-insert at
   `ServerMilliseconds + Delay` and break.
4. **If `Delay == 0`**: pop the entry, execute it (Go/Rotate/Talk/Use/Attack/…), and
   **loop back to step 1** — the next entry is evaluated in the **same heap pop**.

So a monster whose `IdleStimulus` enqueued `ToDoRotate(Target) + ToDoAttack()` with both
`CalculateDelay == 0` (Rotate always has delay 0; Attack is ready) fires **both in one
beat** — rotate and hit on the same 200 ms tick.

Rust `run_monster_todo_execute` (`idle_stimulus.rs:2148`) pops **one** action, executes
it, then calls `finish_creature_todo_execute` (re-arm) or returns. The next action lands
on the **next beat**. A Rotate+Attack sequence:

| | C++ | Rust |
|---|---|---|
| Beat N | Rotate + Attack (one pop) | Rotate only |
| Beat N+1 | (next idle) | Attack only |

**Every melee turn-and-hit takes 400 ms in Rust vs 200 ms in C++.** This is the dominant
"smoothness" gap. It affects every Rotate→Attack, Talk→Act, and ready-Use→Act chain.

**Fix:** make `run_monster_todo_execute` a `while` loop mirroring `Execute`: after each
zero-delay action, check the next head's `CalculateDelay`; if 0, continue in the same
call; if >0, re-insert and break. The `+1` clamp (Finding 17) and `<= server_ms` filter
(Finding 9) already guarantee no same-beat re-entrancy for re-inserted entries, so the
loop terminates.

### 2. Rotate gate blocks combat rotation (AI#22) — compounds #1

Even if the atomic loop were fixed, `monster_update_look_direction` (`monster_ai.rs:2191`)
early-returns when `!walk_timer_idle(beat_driven_loop)`. On 772, `walk_timer_idle` returns
`next_wakeup.is_none()` — and after a Go step the monster always has a pending wakeup, so
the gate blocks the Rotate. C++ `Rotate(Target)` at `crnonpl.cc:2872` runs
**unconditionally** when `State ∈ {ATTACKING, PANIC}` before `ToDoAttack`.

**Fix:** bypass the `walk_timer_idle` gate for
`monster_idle_rotate_toward_attack_target` calls (ATTACKING/PANIC + has attack_target),
or route combat rotation through the ToDo queue as a `TDRotate` entry (which the atomic
loop would then fire zero-delay).

### 3. Missing kick-and-retry in `MovePossible` (AI#23) — the "bump and stall" defect

C++ `MovePossible` (`crnonpl.cc:2185-2288`) has `for(int Attempt = 0; Attempt < 100; …)`:
after kicking a blocking creature/box, it **breaks the inner loop and retries the outer
loop** to re-check whether the tile is now clear. If the kick succeeded and the tile is
empty, the step proceeds **this beat** — no stall.

Rust `monster_kick_before_step_772` (`monster_push.rs:96-179`) kicks once and returns
`Proceed` or `Exhausted`. If the kick moves the blocker but another blocker is behind it
(or the tile needs a second verification), Rust returns `Exhausted` → `Wait(1000)` — a
**1-second stall** where C++ would have retried and stepped through.

This is the "two monsters bump into each other, stop for a second" symptom. The fix is the
`for Attempt in 0..100` retry loop with tile re-check after each kick, matching
`crnonpl.cc:2185-2288`.

### 4. Z-level change clears targets (AI#24) — the "ramp drop" defect

`monster_events.rs:190-198` clears `follow_target` and `attack_target` when
`new_pos.z != old_pos.z`, with no `beat_driven_loop` gate. C++ `CreatureMoveStimulus`
(`crmain.cc:920`, `crnonpl.cc:2943`) does **not** clear targets on Z-change — it only
handles combat re-arm for close-chase. When a player drops down a ramp, Rust monsters lose
their target and must re-acquire via idle stimulus (next beat at best), causing a visible
reactivity lag. C++ monsters keep tracking and follow down.

**Fix:** gate the Z-change target clear to `!beat_driven_loop` (1098 only), or remove it
for 772 entirely and let `CreatureMoveStimulus` handle the re-arm.

### 5. Drunk-walk desyncs RNG (GL#22) — non-determinism + stream poisoning

`walk/mod.rs:116-131` draws from `thread_rng()` (OS entropy) instead of the glibc parity
stream. Every drunk-walk draw doesn't advance the glibc stream, so **every subsequent
`parity_*` draw for that creature desyncs** from C++. This is the "they don't react the
same way twice" symptom for any scenario involving drunk creatures. The algorithm is also
wrong (1098 `r/4 > d` vs 772 `rand()%max(7-level,1)`).

**Fix:** port the 772 `Go` stagger exactly — `max(7-level,1)` gate, `parity_rand_mod`,
two-draw sequence, `ToDoClear/Talk("Hicks!")/Start` re-plan — and source drunkenness from
`ProcessSkills` `Skills[SKILL_DRUNKEN]` (now implemented, Finding 12 FIXED).

### 6. Move-stimulus fan-out order (GL#24) — subtle reactivity ordering

`monster_events.rs:112` sorts witnessing creatures by SlotMap key (creation order). C++
walks the creature list in spatial-sector order. This affects which monster gets the
`CreatureMoveStimulus` first and thus the drain/re-insert order in the ToDoQueue —
observable as "which monster reacts first" in dense packs. Lower priority than #1–#4 but
relevant for sim parity.

## Recommended fix order (smoothness)

1. **Atomic `Execute` loop (GL#7)** — biggest impact, unblocks #2. Make
   `run_monster_todo_execute` a `while` loop over zero-delay entries. Requires
   `CreatureAction::Rotate` (structural root cause).
2. **Rotate gate bypass (AI#22)** — route combat rotation through the ToDo queue
   (`TDRotate`) so the atomic loop fires it zero-delay, or bypass `walk_timer_idle` for
   ATTACKING/PANIC. Subsumed by #1 once `CreatureAction::Rotate` is added.
3. **Kick-and-retry loop (AI#23)** — add `for Attempt in 0..100` with tile re-check to
   `monster_kick_before_step_772`.
4. **Z-level target clear (AI#24)** — gate to `!beat_driven_loop` or remove for 772.
5. **Drunk-walk (GL#22)** — port 772 algo + glibc stream. (GL Phase 8 / AI Phase 5)
6. **Move-stimulus fan-out (GL#24)** — C++ creature-list order. (GL Phase 8)
7. **PANIC→ATTACKING promotion (AI#30)** — move outside early-return guard. (AI Phase 7)
8. **IdleStimulus catch-all (AI#31)** — structural, optional. (AI Phase 7)

Items 1–4 are the felt "smoothness" defects; 5–6 are parity/determinism; 7–8 are low-impact
polish.

---

# Pass 7 — Deep behavioral audit (Jun 2026 re-verification)

Pass 6 identified the smoothness cluster. This pass went deeper into the IdleStimulus
decision tree, the `CreatureMoveStimulus` event path, the `CreatureAction` enum, and the
`process_skills_772` tick details. **Four new behavioral divergences found; one structural
root cause identified.** Combat math, kick direction, kick-kill semantics, path search,
dance/roam distribution, and phase ordering all verified faithful.

## Structural root cause: `CreatureAction` enum is missing Rotate/Talk/Use — blocks GL#7 fix

C++ `Execute` (`cract.cc:783–890`) dispatches on `TToDoEntry.Code`: `TDGo`, `TDRotate`,
`TDTalk`, `TDUse`, `TDTrade`, `TDMove`, `TDTurn`, `TDAttack`, `TDChangeState`. The Rust
`CreatureAction` enum (`creature_todo.rs:65–72`) has only **three** variants:

```rust
pub enum CreatureAction {
    Go,
    Wait { delay_ms: u64 },
    Attack,
}
```

**Missing:** `Rotate`, `Talk`, `Use`, `Trade`, `Move`, `Turn`, `ChangeState`.

This is the **structural reason** the atomic `Execute` loop (GL#7) can't be trivially
fixed and why the Rotate gate (AI#22) exists as a workaround. C++ enqueues
`TDRotate(Target) + TDAttack()` as two ToDo entries; the atomic `Execute` fires both
zero-delay in one beat (200 ms). Rust can't enqueue a Rotate — it does rotation
**out-of-band** via `monster_update_look_direction` (gated by `walk_timer_idle`), then
enqueues only `Attack`. The Rotate and Attack can never be atomic because they live in
different systems.

**Fix path:** add `CreatureAction::Rotate { direction }` (and `Talk` if monster yells are
emitted). Enqueue `Rotate + Attack` from `monster_idle_maybe_enqueue_attack` (matching
`crnonpl.cc:2872`), and let the atomic `Execute` loop fire both zero-delay. This
**subsumes** AI#22 — the `walk_timer_idle` gate becomes irrelevant because rotation is a
ToDo entry, not an out-of-band call.

## Finding 25. Lose-target missing `IsHouse` and invisibility checks — MEDIUM

C++ `IdleStimulus` lose-target (`crnonpl.cc:2418–2435`):

```cpp
bool LoseTarget = (Target == NULL)
    || Target->posz != this->posz
    || std::abs(Target->posx - this->posx) > 10
    || std::abs(Target->posy - this->posy) > 10
    || IsProtectionZone(Target->posx, Target->posy, Target->posz)
    || IsHouse(Target->posx, Target->posy, Target->posz)              // ← missing
    || (Target->IsInvisible() && !RaceData[this->Race].SeeInvisible)  // ← missing
    || (this->Master == 0 && random(0, 99) < RaceData[this->Race].LoseTarget);
```

Rust `monster_idle_772_should_lose_target` (`idle_stimulus.rs:365–399`) checks: dead,
z-diff, distance >10, protection zone, lose-target roll. **Missing:**
- `IsHouse` — monsters don't lose targets that enter houses (should lose them).
- `IsInvisible && !SeeInvisible` — monsters without SeeInvisible don't lose invisible
  targets (should lose them).

**Impact:** a player who enters a house or goes invisible (without the monster having
SeeInvisible) keeps being chased in Rust. In C++ the monster loses target and returns to
idle/roam. Observable in house-bordering and invisibility-rune scenarios.

## Finding 26. `CreatureMoveStimulus` has extra `has_go` check not in C++ — MEDIUM

C++ `CreatureMoveStimulus` (`crmain.cc:920–940`) checks the **head** of the ToDo list:

```cpp
if(this->ActToDo >= this->NrToDo
    || this->ToDoList.at(this->ActToDo)->Code != TDAttack){
    return;
}
```

Rust `monster_combat_creature_move_stimulus` (`monster_events.rs:446`) checks the **entire
queue**:

```rust
if !has_attack || has_go {   // has_go is the extra check
    return;
}
```

**Divergence:** if the queue is `[Attack, Go]` (head is Attack, Go later), C++ fires the
stimulus (head is Attack). Rust doesn't (`has_go` is true). This prevents the combat
re-arm when a monster has a chase step queued after the attack — exactly the close-chase
scenario where the stimulus matters most.

**Fix:** replace `!has_attack || has_go` with a head-specific check: `todo.queue.front()
== Some(&CreatureAction::Attack)`, matching C++'s `ToDoList.at(ActToDo)->Code == TDAttack`.

## Finding 27. `NotifyGo` waypoint cost source mismatch — LOW (verify data)

C++ `NotifyGo` (`cract.cc:1513–1534`) reads the `WAYPOINTS` attribute from the
destination tile's `BANK` object and multiplies by 3 for diagonal:

```cpp
int Waypoints = BankType.getAttribute(WAYPOINTS);
if(DiagonalMove){ Waypoints *= 3; }
int Delay = (Waypoints * 1000) / this->GetSpeed();
```

Rust `walk/walk_timing.rs:184–218` uses `ground_speed` from `items_db` multiplied by a
fixed waypoint cost (3 diagonal, 1 cardinal):

```rust
let gs = if ground_speed == 0 { 150 } else { ground_speed };
let waypoints = gs.saturating_mul(waypoint_cost.max(1));
let delay = (waypoints as i64 * 1000) / i64::from(eff.max(1));
```

**Potential divergence:** if `ground_speed` (from `items_db`) ≠ `WAYPOINTS` (from tile
`BANK` attribute), step delays differ. The formula structure matches (`waypoints * 1000 /
speed`, ceil to Beat). Need to verify that `items_db.ground_speed` maps 1:1 to the C++
`WAYPOINTS` attribute for all tile types. If they match for shipped data, this is latent
only.

## Finding 28. Poison field extension missing in `process_skills_772` — LOW

C++ `TSkillPoison::Event` (`crskill.cc:1020–1048`) extends poison duration when the
creature is standing on a poison field:

```cpp
Object Obj = GetFirstObject(Master->posx, Master->posy, Master->posz);
while(Obj != NONE){
    if(ObjType.getFlag(AVOID) && ObjType.getAttribute(AVOIDDAMAGETYPES) == DAMAGE_POISON){
        this->Cycle += 1;   // extend poison duration
    }
    Obj = Obj.getNextObject();
}
```

Rust `process_skills.rs:74–86` ticks poison damage and decays `total_rank` by 50% per
tick, but **does not check for poison fields under the creature**. A creature standing on
a poison field in Rust has its poison wear off on schedule; in C++ the field re-extends
the duration. Observable in poison-field kiting scenarios.

## Verified faithful (this pass)

- **IdleStimulus phase ordering** — lose-target → state normalize → talk → target acquire
  → cast → walk → ToDoStart. Exact match (`idle_stimulus.rs:984–1042` vs
  `crnonpl.cc:2418–2887`).
- **Target acquisition (SearchEnemy)** — aggro radius 10, `CanSeeFloor`, player-controlled
  filter, `parity_random(0,99)` strategy + tiebreaker. Match.
- **State transitions** — PANIC trigger (no target → PANIC), UnderAttack (target + IDLE),
  fleeing computed from HP threshold. Match.
- **ToDoGo step budget** — melee chase 3, distance chase `cheb - target_distance`, roam
  `INT_MAX`. Match.
- **TShortway path search** — reverse dest→origin, VisibleX/Y=10, 441 node cap. Match.
- **Dance direction** — `rand()%5`, W/E/N/S/hold, 1-in-5 stay. Match.
- **Roam** — `rand()%4` cardinals, 10-try cap. Match.
- **ToDoWait(1000)** after dance/roam/EXHAUSTED. Match.
- **Kick direction** — fixed N,S,W,E, no RNG. Match.
- **Kick-kill** — full HP damage, kicker in damage_map, death pipeline. Match.
- **KickBoxes** — UNPASS/AVOID, fixed order, BANK dest, delete on failure. Match.
- **Combat damage** — `((rand%100+rand%100)/2)*Max/10000`, `Max=attack*(skill*5+50)`.
  Match.
- **Fight mode modifiers** — +20/−40 atk, −40/+80 def. Match.
- **DelayAttack** — 200ms (fast/initial), 2000ms (cadence). Match.
- **Condition ticks** — fire 10/8, energy 25/10, poison 50% decay. Match.
- **XP distribution** — proportional from damage_map, PvP cap. Match.
- **LockToDo** — checked before pop, set after. Match.
- **ToDoStart/Yield/Wait** — `+1` clamp, `ToDoWait(0)+Start`, absolute→relative time.
  Match.

## Pass 7 verdict additions

The deep pass confirms the **decision tree is faithful** — phase ordering, target
acquisition, dance/roam, combat math, kick semantics all match. The remaining defects are
concentrated in three areas:

1. **The `CreatureAction` enum gap** (structural) — missing Rotate/Talk/Use blocks the
   atomic `Execute` fix and forces the out-of-band Rotate workaround (AI#22).
2. **Lose-target completeness** (AI#25) — missing house + invisibility checks.
3. **MoveStimulus head check** (AI#26) — `has_go` over-gates vs C++ head-only check.

These are additive to the Pass 6 smoothness cluster. The recommended fix order updates:

7. **Add `CreatureAction::Rotate`** + enqueue Rotate+Attack from idle (subsumes AI#22,
   unblocks GL#7).
8. **Fix lose-target: add `IsHouse` + invisibility** (AI#25).
9. **Fix MoveStimulus: head-only check** (AI#26).
10. **Poison field extension** (AI#28) — low priority.
11. **Waypoint cost source verification** (AI#27) — verify `ground_speed` == `WAYPOINTS`.
12. **PANIC→ATTACKING promotion gating** (AI#30, Phase 7) — move promotion outside early-return. Low.
13. **IdleStimulus catch-all coverage** (AI#31, Phase 7) — structural, not urgent. Low.

## Finding 30. PANIC→ATTACKING promotion only fires on successful dance step — LOW

C++ `crnonpl.cc:2832–2834` promotes `PANIC → ATTACKING` **unconditionally** inside the
melee band (`Distance == 1`), regardless of whether `MovePossible` succeeded for the dance
step:

```cpp
if(DestDistance == 1 && this->MovePossible(DestX, DestY, DestZ, true, false)){
    this->ToDoGo(DestX, DestY, DestZ, true, INT_MAX);
}
if(this->State == PANIC){
    this->State = ATTACKING;   // ← fires even if MovePossible failed
}
```

Rust `monster_idle_dance_step` (`monster_ai.rs:1349–1381`) returns `false` early at line
1353 when `!monster_can_walk_to(...)`, **before** reaching the promotion at line 1374. The
promotion only fires when the dance step is valid (walkable or hold).

**Impact:** LOW — a PANIC monster cornered at melee range (all dance steps blocked) stays
in PANIC in Rust but promotes to ATTACKING in C++. The immediate behavior is similar because
`crnonpl.cc:2872` treats both PANIC and ATTACKING the same for `Rotate + ToDoAttack`. The
state difference persists into the next idle stimulus, but `IsFleeing()` is HP-based (not
state-based), so spell selection is unaffected. The main observable difference would be in
state-transition edge cases (e.g., target loss from PANIC → IDLE vs ATTACKING → different
path).

**Fix:** Move the `band == 1` PANIC→ATTACKING promotion outside the early-return guard, or
have the caller perform the promotion when the dance step fails at melee range.

## Finding 31. `IdleStimulus` catch block — partially covered, not comprehensive — LOW

C++ `TMonster::IdleStimulus` wraps the walk/attack section in a try/catch
(`crnonpl.cc:2750–2898`). The catch block at `crnonpl.cc:2890–2898` fires on **any**
exception:

```cpp
}catch(RESULT r){
    this->Target = 0;
    this->ToDoClear();
    if(r == EXHAUSTED){
        this->ToDoWait(1000);
        this->ToDoStart();
        return;
    }
}
```

Rust has `monster_exhausted_wait_772` (`idle_stimulus.rs:152–164`) which replicates the
EXHAUSTED branch (clear targets + todo + Wait(1000) + Start). It is called from:
- `monster_push.rs:589` — kick failure (EXHAUSTED equivalent)
- `walk/mod.rs:1271` — walk execution failure

**Divergence:** The Rust handler is called from **specific failure points** (push, walk)
rather than being a **comprehensive catch-all** like the C++ try/catch. Any new failure path
in the walk/attack section that doesn't explicitly call `monster_exhausted_wait_772` would
leave the monster in an inconsistent state (target retained, todo not cleared). In C++, the
catch block guarantees cleanup on **any** exception from the entire walk/attack section.

**Impact:** LOW — the two known failure paths (push/kick and walk) are covered. The risk is
future divergence if new failure paths are added without the handler. A structural fix
(making the idle stimulus return a `Result` and handling all errors at the call site) would
match the C++ catch-all semantics, but is not urgent given current coverage.

---

## Out of plan (revisit only if in scope)

`TNPC::IdleStimulus` (NPC AI), `MovePossible` field-`AVOID` cross-check (fire/poison/energy
avoidance, PANIC hazard ignore), `SearchSummonField` summon placement, and 772 convince/challenge
rune semantics (Finding 21) — none are on the reported melee/distance "feel" path.
