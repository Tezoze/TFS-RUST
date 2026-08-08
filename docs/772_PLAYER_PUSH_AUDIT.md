# 772 Player Push (Creature Drag) — Parity Audit + Implementation Plan

**Audited:** 2026-08-08
**Re-verified:** 2026-08-08 — all decompile citations and Rust claims re-checked against source;
three corrections applied (V1: P-D kick loop, V2: `Radius` is per-creature not race, V3: TFS
`canPushCreatures` conflation). Open questions Q1–Q4 resolved (§6).
**Re-verified (2nd pass):** 2026-08-08 — seven further corrections applied after a second source
pass (C1–C7, §7). C1 (P5 height gate) is material: 772 `GetHeight` is an **elevation sum**, not a
hasHeight **count**; the existing `tile_has_height_n` is the wrong metric, inverted, and has no
`dz` condition, so the current gate is dead code and **every** floor-change push passes. P-C as
written would perpetuate all three bugs. C2/C3 (P-D signatures) require `Result<bool, ReturnValue>`
because `TPlayer::MovePossible` and the monster kick loop **throw** (`ENTERPROTECTIONZONE`,
`NOTINVITED`, `EXHAUSTED`) rather than return `false`. C7 adds three findings the audit missed
(`CheckTopMoveObject`, execute-time `ObjectAccessible`, `SeparationEvent`/`MovementEvent`/
`CollisionEvent`). See §7 for the full correction list.
**Reference (772 mechanics):** `reference/cipsoft-772/tibia-game-master/src/` — `operate.cc`,
`cract.cc`, `crmain.cc`, `crnonpl.cc`, `crplayer.cc`, `info.cc`, `map.cc`, `receiving.cc`,
`enums.hh`, `cr.hh`.
**Reference (772 wire):** `reference/tvp-772/gameserver/src/` — not used here (no packet changes;
the throw/move-object packet is already wired via `GamePacket::Throw`).

**Rust files audited:**

| Rust file | 772 counterpart |
|---|---|
| `game_world_player_throw.rs` | `cract.cc:475` `TCreature::Move`, `receiving.cc:233` `CMoveObject` |
| `creature_todo.rs` | `cract.cc:1123` `TCreature::ToDoMove` (queue builder) |
| `idle_stimulus.rs` | `cract.cc:823` `Execute` `TDMove` → `execute_player_move` |
| `game_loop.rs` | `receiving.cc:233` `CMoveObject` packet dispatch → `enqueue_player_move` |
| `creature/monster.rs` | `crmain.cc:1356` `GetRaceUnpushable`, `crmain.cc:1495` race flag parse |
| `creature/base.rs` | `cr.hh:631` `EarliestWalkTime`, `cr.hh` `Master` |
| `walk/walk_tile.rs` | `info.cc:702` `JumpPossible`, `crmain.cc:883` `TCreature::MovePossible` |
| `walk/mod.rs` | `cract.cc:1566` `NotifyTurn`, `cract.cc:1400` `NotifyGo` |
| `map/los.rs` | `info.cc:1154` `ThrowPossible` |

**Data-vs-mechanics split (unchanged decision):** race flags (`unpushable`, `kickcreatures`,
`kickboxes`, `seeinvisible`) come from the TFS monster XML / OTB-equivalent race data already loaded
into `Monster` (`creature/monster.rs`). Per-creature fields `Radius` (`crmain.cc:174`, default
`INT_MAX`; set from spawn-wave `Wave->Radius` at `crmain.cc:1886,2061`) and `Home`
(`crnonpl.cc:2008`) are **not** race flags — they're on `TCreature` directly. The Rust `Monster`
models `home_radius` (`creature/monster.rs:185`, the `MonsterhomeInRange` roam radius) but **not**
the per-creature `Radius` (max distance from current position); P-D must add it or default to
`i32::MAX`. The mechanics layer is the 772 decompile: the pushability predicate, the per-creature
`MovePossible` constraints, the `CheckMapDestination` creature-container arm, and the `ToDoMove`
1000 ms + walk-cooldown delay.

**Data-pack caveat (V3):** the TFS data pack conflates 772's independent `unpushable` and
`kickcreatures` race flags — `canPushCreatures="1"` forces `pushable=false`
(`tfs-rust-content/src/monsters.rs:778-780`, mirroring TFS `monsters.cpp ~997-1000`). So
`race_unpushable() = !self.pushable` is correct for the existing data pack, but a 772-accurate
monster with `kickcreatures` but pushable cannot be expressed. Document in `tasks/lessons.md`.

---

## 0. Verdict summary

The player-push path exists and handles the **happy path** (pushable monster one tile over on the
same floor, no PZ boundary, no hazard): `DelayAttack(2000)`, `NotifyTurn`, `AnnounceMovingCreature`
(0x6D), `MoveObject`, `NotifyGo`, PZ→non-PZ reject, and a height-24 check are all present. **C1
correction:** the height-24 check is dead code (wrong metric, inverted, no `dz` guard), so even the
happy path for floor changes is wrong — see P5. The defects cluster around the **gate logic**
(pushability model, per-creature `MovePossible`, the 1-tile destination cap, the `AVOID`-on-dest
rule, the top-move-object rule, execute-time adjacency) and the **pacing** (the 1000 ms +
walk-cooldown `ToDoMove` creature-container delay is not ported — already tracked as "D9").

| # | Finding | Severity | Outcome differs? |
|---|---------|----------|------------------|
| P1 | `ToDoMove` creature-container delay (1000 ms + walk cooldown) not ported | **High** | Yes — pushes fire at ~100 ms instead of ~1000 ms |
| P2 | Pushability uses TFS `is_pushable()` (`pushable && speed != 0`) instead of 772 `!GetRaceUnpushable \|\| (NON_PVP && IsPeaceful)` | **High** | Yes — `speed==0` monsters unpushable in Rust; NON_PvP peaceful-summon exception missing |
| P3 | No 1-tile destination range cap (`\|Δx\|>1 \|\| \|Δy\|>1 \|\| \|Δz\|>1 → OUTOFRANGE`) | **High** | Yes — can push 2+ tiles away |
| P4 | No per-creature `MovePossible` on the moving creature (home range, radius, house, PZ-enter, GO_STRENGTH, summon anti-crowd) | **High** | Yes — pushes monsters out of home range / into houses / into PZ |
| P5 | Height-24 gate is **dead code**: uses `tile_has_height_n` (a hasHeight **count** ≥ 24, never true) instead of 772 `GetHeight` (an **elevation sum** `< 24`); has no `dz` condition so it nominally applies to same-floor pushes too; and `Elevation` is not parsed anywhere in `crates/` | **High** | Yes — **every** floor-change push passes; low-origin creature can be pushed up; low-dest down push passes |
| P6 | No dest `AVOID` flag → `NOROOM` rule when pushing another creature | Medium | Yes — can push onto a magic-field tile |
| P7 | NPC pushability hardcoded `false` instead of race-driven | Low | Edge — practically all NPC races are `unpushable`, but the model is wrong |
| P8 | `BANK` dest first-object check (`cract.cc:1145`) not explicit (partially covered by `tile_query_add_creature` ground check) | Low | Edge |
| P9 | `ThrowPossible(..., 1)` not called on the push destination | Medium | Yes — at range 1 it tests `UNTHROW` on the destination tile and runs the floor-descent loop for `dz != 0`; not trivially true (C6) |
| P10 | `IsPeaceful` (player-summon predicate) not modeled as a shared helper | Low | Refactor — needed by P2 and by monster AI |
| P11 | `CheckTopMoveObject` (`operate.cc:1356`) not implemented — only the first creature in the tile stack is eligible to be pushed; a second creature behind it throws `NOTACCESSIBLE` | Medium | Yes — can push a creature that is not the top move object |
| P12 | Execute-time `ObjectAccessible(CreatureID, Obj, 1)` re-check (`operate.cc:424` inside `CheckMoveObject`) not ported — Rust only checks actor↔target adjacency at enqueue (`object_in_range` in `enqueue_player_move`). **Gets worse with P-B**: the 1000 ms wait lets the target walk away; 772 rejects at execute, Rust still pushes | Medium | Yes — target can walk out of range during the wait and still be pushed |
| P13 | `SeparationEvent(Obj, OldCon)` (before `MoveObject`, `operate.cc:1386`) and `MovementEvent`/`CollisionEvent`/`NotifyAllCreatures(OBJECT_MOVED)` (after) not audited for the push path — §1.4 lists them but §3.1 never checks whether `flush_pending_creature_step_events` / `apply_notify_go_after_relocate` cover trap/teleporter/field-on-destination side effects for pushed creatures | Low | Unknown — needs audit |

---

## 1. Decompile flow — the full push pipeline

### 1.1 Packet → queue

`receiving.cc:233` `CMoveObject` reads the throw packet
(`OrigX/Y/Z`, `TypeID`, `RNum`, `DestX/Y/Z`, `Count`), rejects `Type.isMapContainer()` (sprite 0)
and `CUMULATIVE && Count == 0`, runs `CheckSpecialCoordinates` / `CheckVisibility`, then calls
`Player->ToDoMove(OrigX, OrigY, OrigZ, Type, RNum, DestX, DestY, DestZ, Count)` + `ToDoStart()`.

### 1.2 `ToDoMove` — the queue builder (`cract.cc:1123`)

```
TCreature::ToDoMove(ObjX, ObjY, ObjZ, Type, RNum, DestX, DestY, DestZ, Count):
  Obj = GetObject(this->ID, ObjX, ObjY, ObjZ, RNum, Type)   // resolve now
  if !Obj.exists() -> NOTACCESSIBLE
  if ObjX != 0xFFFF:                                         // map source
    if this->posz > ObjZ -> UPSTAIRS
    if this->posz < ObjZ -> DOWNSTAIRS
    if !ObjectInRange(this->ID, Obj, 1):
      this->ToDoGo(ObjX, ObjY, ObjZ, false, INT_MAX)         // walk to source first
  Delay = 100
  if Obj.getObjectType().isCreatureContainer():
    DestBank = GetFirstObject(DestX, DestY, DestZ)
    if DestBank == NONE || !DestBank.getObjectType().getFlag(BANK):
      throw NOTACCESSIBLE                                    // dest must have a bank (ground)
    Creature = GetCreature(Obj)
    Delay = 1000
    if this->EarliestWalkTime > ServerMilliseconds:
      Delay += (int)(this->EarliestWalkTime - ServerMilliseconds)   // + walk cooldown
  this->ToDoWait(Delay)
  TD.Code = TDMove; TD.Move = {Obj, DestX, DestY, DestZ, Count}
  this->ToDoAdd(TD)
```

**Key:** the creature-container branch sets `Delay = 1000` **and** adds the remaining walk
cooldown. The non-creature (item) branch uses `Delay = 100`. The Rust port currently always uses
`100` (P1).

### 1.3 `Execute` `TDMove` (`cract.cc:823`) → `TCreature::Move` (`cract.cc:475`)

```
case TDMove: this->Move(Object(TD.Move.Obj), TD.Move.x, y, z, Count)
```

`TCreature::Move`:
- `Obj == this->CrObject` → `this->Go(Dest)` (self-walk, not a push).
- `ObjType.isCreatureContainer()` → `this->Combat.DelayAttack(2000)` (the **actor** gets a 2 s
  attack cooldown after pushing).
- `DestX == 0xFFFF` → inventory/container destination (item path, not push).
- else (map destination): `Dest = GetMapContainer(DestX, DestY, DestZ)`; HANG+hook out-of-range →
  walk-to-reach re-enqueue; else `::Move(this->ID, Obj, Dest, MoveCount, false, NONE)`.

### 1.4 `::Move` (`operate.cc:1282`) — the global move

For a creature-container object moving to a map container, the relevant checks are:

1. `CheckTopMoveObject(CreatureID, Obj, Ignore)` — top-object rule (only players).
2. `CheckMoveObject(CreatureID, Obj, false)` — **the pushability gate (Gate A)**.
3. `CheckMapDestination(CreatureID, Obj, Con)` — **destination validation (Gate C)**.
4. `MoveObject(Obj, Con)` — `CutObject` + `PlaceObject` (`map.cc:2119`).
5. `NotifyTurn(Con)` → `AnnounceMovingCreature` → `NotifyGo` (creature-specific side effects).
6. `MovementEvent` / `CollisionEvent` / `NotifyAllCreatures(OBJECT_MOVED)`.

### 1.5 `MoveObject` side effects for creatures (`operate.cc:1414-1446`)

```
if ObjType.isCreatureContainer():
  Creature = GetCreature(Obj.getCreatureID())
  Creature->NotifyTurn(Con)            // direction-only, no 0x6B
  AnnounceMovingCreature(MovingCreatureID, Con)   // 0x6D to spectators
MoveObject(Obj, Con)
if ObjType.isCreatureContainer():
  Creature->NotifyGo()                 // post-move hook
```

`NotifyTurn` (`cract.cc:1566`) sets `Direction` from the delta (E/W/N/S), no broadcast.
`AnnounceMovingCreature` (`operate.cc:31`) searches a player rectangle around the midpoint of
origin/dest and sends `SendMoveCreature` (0x6D).

---

## 2. The three pushability gates (decompile)

### Gate A — race flag (`operate.cc:439` `CheckMoveObject`)

```cpp
if(ObjType.isCreatureContainer() && Obj.getCreatureID() != CreatureID){
  TCreature *MovingCreature = GetCreature(Obj);
  if(GetRaceUnpushable(MovingCreature->Race)
     && (WorldType != NON_PVP || !MovingCreature->IsPeaceful())){
    throw NOTMOVABLE;
  }
}
```

- `GetRaceUnpushable(Race)` = `RaceData[Race].Unpushable` (`crmain.cc:1356`), set by the
  `unpushable` race flag in monster XML (`crmain.cc:1495`).
- `IsPeaceful`:
  - base `TCreature::IsPeaceful` = `true` (`crmain.cc:900`) — covers **players** and **NPCs**.
  - `TMonster::IsPeaceful` = `Master != 0 && IsCreaturePlayer(Master)` (`crnonpl.cc:2295`) — a
    player-owned summon is peaceful.
- **Net rule:** in `NON_PVP` worlds, an `unpushable`-race creature that is *peaceful* (player,
  NPC, or player-summon) **can** be pushed. In `NORMAL` / `PVP_ENFORCED`, any `unpushable`-race
  creature is unpushable. A non-`unpushable`-race creature is always pushable (subject to Gate B/C).
- NPCs are **not** type-special-cased here — they go through the same race lookup. Practically all
  NPC races are `unpushable`, so NPCs are usually unpushable except in NON_PVP (where the peaceful
  exception lets them be pushed).

### Gate B — destination `MovePossible` (`operate.cc:515-516`)

`CheckMapDestination` calls `MovingCreature->MovePossible(DestX, DestY, DestZ, Execute=true,
OrigZ != DestZ)` on the **moving** creature. Per type:

| Creature | `MovePossible` ref | Constraints |
|----------|--------------------|-------------|
| Player | `crplayer.cc:363` | base (`JumpPossible`/`BANK&&!UNPASS`) + PZ-enter gate (`EarliestProtectionZoneRound`) + house-invite gate |
| Monster | `crnonpl.cc:2141` | same-z; home-range (unless ATTACKING/PANIC); `Radius`; `GO_STRENGTH ≥ 0`; `!PZ`; `!House`; summon anti-crowd rule (C4); then kick blocking creatures/boxes on dest |
| NPC | `crnonpl.cc:1672` | `BANK && !UNPASS && !AVOID && z==startz && within Radius && !House` |
| Base | `crmain.cc:883` | `Jump` → `JumpPossible`; else `BANK && !UNPASS`; `!Execute && AVOID` → false |

`JumpPossible` (`info.cc:702`): tile has a `BANK` object, no `UNPASS && UNMOVE` object, and (if
`AvoidPlayers`) no player creature.

**C4 — "summon-leash" is a mischaracterization:** `crnonpl.cc:2171-2181` is the **inverse** of a
leash. Given `Execute && Master != 0 && State ∉ {ATTACKING, PANIC}` and master on the same z, it
rejects when the summon is currently **not** adjacent (manhattan > 1) and the destination **would
be** adjacent (≤ 1) — an anti-crowding rule that stops a summon from snapping onto the master's
tile. It is **not** a distance cap on how far a summon can be pushed from its master.
Implementing "leash" (distance cap) would be wrong; implement the anti-crowd predicate exactly.

### Gate C — destination tile (`operate.cc:493-532`, creature-container arm of `CheckMapDestination`)

```
if std::abs(OrigX - DestX) > 1 || std::abs(OrigY - DestY) > 1 || std::abs(OrigZ - DestZ) > 1:
  throw OUTOFRANGE                              // PUSH IS 1-TILE ONLY
if DestZ == OrigZ - 1:                          // up a floor
  if GetHeight(OrigX, OrigY, OrigZ) < 24:       // checks ORIGIN elevation sum
    throw NOROOM
else if DestZ == OrigZ + 1:                     // down a floor
  if GetHeight(DestX, DestY, DestZ) < 24:       // checks DEST elevation sum
    throw NOWAY
MovingCreature = GetCreature(Obj)
if OrigZ == DestZ || MovingCreature->Type != MONSTER:
  if !MovingCreature->MovePossible(Dest, Execute=true, OrigZ != DestZ):
    if CreatureID == MovingCreature->ID: throw MOVENOTPOSSIBLE
    else: throw NOROOM
  if CreatureID != MovingCreature->ID:          // pushing ANOTHER creature
    if CoordinateFlag(Dest, AVOID): throw NOROOM
    if IsProtectionZone(Orig) && !IsProtectionZone(Dest): throw PROTECTIONZONE
// HANG hook destination range (omitted — item path)
if !ThrowPossible(Orig, Dest, 1): throw CANNOTTHROW
```

**`GetHeight` is an elevation sum, not a count (C1):** `GetHeight` (`info.cc:689`) iterates the
tile stack and sums the `ELEVATION` **attribute** of every `HEIGHT`-flagged object:
```cpp
int GetHeight(int x, int y, int z){
  int Result = 0;
  Object Obj = GetFirstObject(x, y, z);
  while(Obj != NONE){
    ObjectType ObjType = Obj.getObjectType();
    if(ObjType.getFlag(HEIGHT)){
      Result += (int)ObjType.getAttribute(ELEVATION);
    }
    Obj = Obj.getNextObject();
  }
  return Result;
}
```
The gate is `sum < 24`. Rust `tile_has_height_n` (`walk/walk_tile.rs:39`) is TFS
`Tile::hasHeight(n)` — a **count** of hasHeight items, true at `count >= n`. These are different
metrics. `Elevation` is **not parsed anywhere in `crates/`** (0 matches for `elevation`/`ELEVATION`
in `crates/`), so the gate cannot be ported correctly until the OTB `Elevation` attribute is added
to the item model. **P5 prerequisite:** add `Elevation` parsing before P-C can land the height
gate.

**Key asymmetries:**
- Up-floor checks **origin** elevation sum; down-floor checks **dest** elevation sum. (Rust checks
  dest for both, uses the wrong metric, and has no `dz` condition — P5.)
- `AVOID` on dest only blocks when pushing **another** creature (not self-walk). (Rust missing —
  P6.) **C5:** in the decompile, `AVOID` and `PZ → non-PZ` are nested **inside**
  `if(OrigZ == DestZ || Type != MONSTER)`, **after** `MovePossible` succeeds — they only run when
  `MovePossible` was called. The proposed `check_push_destination` helper runs them unconditionally
  and before `MovePossible`; the helper must mirror the nesting or it will reject pushes that 772
  accepts (e.g. a monster changing floors where `MovePossible` is skipped).
- `PZ → non-PZ` only blocks when pushing **another** creature. Self-walk PZ-enter is gated by
  `TPlayer::MovePossible` instead.
- `MovePossible` is skipped for monsters changing floors (`OrigZ != DestZ && Type == MONSTER`) —
  monsters can't change floors anyway, so this is a no-op in practice but structurally distinct.
- **C6:** `ThrowPossible(…, 1)` is **not** trivially true at range 1. It iterates `T = 1..MaxT`
  testing `UNTHROW` on the destination tile, plus a floor-descent loop for `dz != 0`. At range 1
  (`MaxT == 1`) it tests `UNTHROW` on the dest tile once; with `dz != 0` it also scans floors
  between origin and dest. It is a real gate (P9 is Medium, not Low).

---

## 3. Rust implementation status

### 3.1 `player_push_creature` (`game_world_player_throw.rs:119`)

| # | Decompile behavior | Rust status |
|---|--------------------|-------------|
| 1 | `DelayAttack(2000)` on actor | ✅ lines 187-188 |
| 2 | `NotifyTurn` + `AnnounceMovingCreature` (0x6D) + `MoveObject` + `NotifyGo` | ✅ lines 170-183 |
| 3 | PZ→non-PZ reject (pushing another creature) | ✅ line 158-160 |
| 4 | Height-24 gate (elevation sum `< 24`; origin for up, dest for down) | ❌ P5 — **dead code**: `tile_has_height_n` is a hasHeight count ≥ 24 (never true), no `dz` condition, and `Elevation` is not parsed (C1) |
| 5 | Pushability = `!GetRaceUnpushable \|\| (NON_PVP && IsPeaceful)` | ❌ P2 — uses TFS `is_pushable()` = `pushable && speed != 0` |
| 6 | NPC pushability = race-driven | ❌ P7 — hardcoded `Npc(_) => false` |
| 7 | 1-tile destination range (`\|Δ\|>1 → OUTOFRANGE`) | ❌ P3 — not enforced |
| 8 | Dest `AVOID` → `NOROOM` (pushing another creature, nested after `MovePossible`) | ❌ P6 |
| 9 | Per-creature `MovePossible` (home range, radius, house, PZ-enter, GO_STRENGTH, summon anti-crowd) | ❌ P4 — `tile_query_add_creature` covers tile occupancy only |
| 10 | `Delay = 1000` + walk-cooldown remainder (`cract.cc:1156-1159`) | ❌ P1 — `enqueue_player_move` always uses `Wait{100}` |
| 11 | `BANK` dest first-object (`cract.cc:1145`) | ⚠️ P8 — partially covered by `tile_query_add_creature` ground check |
| 12 | `ThrowPossible(..., 1)` — tests `UNTHROW` on dest + floor-descent loop | ❌ P9 — not called (C6: not trivially true) |
| 13 | `CheckTopMoveObject` (`operate.cc:1356`) — only first creature in tile stack is pushable; else `NOTACCESSIBLE` | ❌ P11 — not implemented (C7) |
| 14 | Execute-time `ObjectAccessible(CreatureID, Obj, 1)` (`operate.cc:424` in `CheckMoveObject`) — actor↔target adjacency re-check at execute | ❌ P12 — only checked at enqueue; gets worse with P-B's 1000 ms wait (C7) |
| 15 | `SeparationEvent` (before `MoveObject`) + `MovementEvent`/`CollisionEvent`/`NotifyAllCreatures(OBJECT_MOVED)` (after) for pushed creatures | ⚠️ P13 — not audited for push path; `flush_pending_creature_step_events`/`apply_notify_go_after_relocate` coverage unverified (C7) |

### 3.2 `enqueue_player_move` (`creature_todo.rs:577`)

Already flagged as unported "D9" at lines 572-573, 605: the creature-container branch (`Delay =
1000` + `BANK` dest check + walk-cooldown remainder) is not implemented. The current code always
calls `enqueue_creature_wait(cid, 100)`.

### 3.3 Primitives that already exist (reusable)

| Primitive | Rust location | Notes |
|---|---|---|
| `EarliestWalkTime` | `creature/base.rs:228` `earliest_walk_server_ms` | Used by walk beat; reusable for P1 delay |
| `WorldType` | `tfs_rust_common::WorldType` (`NoPvp`/`Pvp`/`PvpEnforced`) | `config.rs:466`, exposed to Lua at `game_world_script.rs:818` |
| `is_summon` / `master` | `creature/base.rs:294`, `:252` | For `IsPeaceful` monster arm |
| `tile_in_protection_zone` | `player/combat/mod.rs:793` | For PZ gates |
| `throw_possible` | `map/los.rs:87` | For P9 |
| `tile_has_height_n` | `walk/walk_tile.rs:39` | **Not reusable for P5 (C1):** this is TFS `Tile::hasHeight(n)` — a hasHeight **count** ≥ n. 772 `GetHeight` is an **elevation sum** `< 24` (`info.cc:689`). P5 needs a new `tile_elevation_sum(pos)` helper, and `Elevation` must be parsed from OTB first (0 matches in `crates/` today). |
| `object_in_range` | `game_world_item_cylinder.rs:332` | For P3 |
| `Monster::pushable` | `creature/monster.rs:198` (struct field; config origin `:47`) | The race `unpushable` flag (negated) — **this is the 772 `Unpushable` race flag**, already loaded |
| `tilestate::MAGICFIELD` / `AVOID` mapping | `walk/walk_tile.rs:660` (comment `:656`), `monster_push.rs` | For P6 dest-AVOID check |
| `NotifyTurn` (state-only) | `walk/mod.rs:389` `set_direction_from_step_for_kick` (doc `:388`) | Already used by push |
| `AnnounceMovingCreature` | `broadcast_spectator_move` (called at line 176) | Already used by push |

---

## 4. Implementation plan

Phased so each phase is independently verifiable. P1 (pacing) and P2 (pushability) are the
highest-impact behavior divergences; P3/P4 are the highest-impact correctness gaps. All phases
preserve the TFS-shaped domain (`Move`/`MoveObject` entry points, `Creature` trait surface) and
put era knobs in `MechanicsProfile` / formulas where applicable.

### Phase P-A — Pushability predicate (P2, P7, P10) [High]

**Goal:** replace `is_pushable()` with the 772 `CheckMoveObject` race-flag predicate, including
the NON_PVP peaceful exception.

**Files:**
- `creature/monster.rs` — add `fn race_unpushable(&self) -> bool` (= `!self.pushable`, the loaded
  race `Unpushable` flag). Keep `is_pushable()` for TFS-style monster-AI use (kick gate) but do
  **not** use it for the player-push gate.
- `game_world.rs` (or a small `creature/peaceful.rs`) — add a shared `is_peaceful(cid)` helper:
  - Player → `true`
  - NPC → `true`
  - Monster → `master.is_some() && master is Player`
  Cite `crmain.cc:900` (base), `crnonpl.cc:2295` (`TMonster::IsPeaceful`).
- `game_world_player_throw.rs:129-136` — replace the pushability block:
  ```rust
  let unpushable = match target {
      CreatureKind::Monster(m) => m.race_unpushable(),
      CreatureKind::Player(_) | CreatureKind::Npc(_) => false, // players/NPCs have no race flag
  };
  // 772 `CheckMoveObject` (`operate.cc:439`): unpushable race blocks unless NON_PVP && peaceful.
  if unpushable && !matches!(self.pvp_config.world_type, WorldType::NoPvp) {
      return Err(ReturnValue::NotMoveable);
  }
  if unpushable && !self.is_peaceful(moving_creature) {
      return Err(ReturnValue::NotMoveable);
  }
  ```
  **Note:** players/NPCs have no `Race` in the 772 sense for this gate — they're never
  `GetRaceUnpushable`-blocked. The hardcoded `Npc(_) => false` is removed (P7): NPC pushability
  is now governed by Gate B/C (`MovePossible`), matching the decompile.

  **Observable message change (minor nit):** the current Rust code returns `ReturnValue::NotPossible`
  ("Sorry, not possible.") for the unpushable case. 772 throws `NOTMOVABLE`, which maps to
  `ReturnValue::NotMoveable` ("You cannot move this object."). P-A changes the client-visible
  message. This is correct per 772 parity but is an observable behavior change — call it out in the
  PR and in `tasks/lessons.md`.

**Verify:** `cargo test` — add a unit test: NON_PvP world, player-summon of an `unpushable` race
is pushable; same summon in PVP world is not. A normal (`!unpushable`) monster is pushable in all
world types.

### Phase P-B — `ToDoMove` creature-container delay (P1) [High]

**Goal:** port the `cract.cc:1144-1160` creature-container branch of `ToDoMove` so pushes are
paced at 1000 ms + walk-cooldown remainder.

**Files:**
- `creature_todo.rs:577` `enqueue_player_move` — branch on whether the resolved source object is a
  creature container:
  ```rust
  let is_creature_container = self.creatures.get(cid).is_some_and(|k| {
      // The *source object* is a creature iff the resolved thing is a Creature.
      // `enqueue_player_move` currently doesn't resolve the thing; either resolve it here
      // (mirror `internal_get_thing_move`) or thread an `is_creature` flag from the caller.
  });
  let delay = if is_creature_container {
      // 772 `cract.cc:1145-1148`: dest must have a BANK first object (ground tile).
      // `ReturnValue::NotAccessible` doesn't exist in the Rust enum — use `NotPossible`
      // (matching existing `validate_move_object_ref` / `player_push_creature` patterns).
      let Some(to_tile) = self.map.get_tile(dest_if_map) else {
          return Err(ReturnValue::NotPossible);
      };
      if to_tile.body().ground.is_none() { return Err(ReturnValue::NotPossible); }
      // 772 `cract.cc:1156-1159`: 1000 ms + remaining walk cooldown.
      let mut d = 1000;
      let base = self.creatures.get(cid).map(|k| k.base());
      if let Some(b) = base {
          if b.earliest_walk_server_ms > self.server_ms {
              d += (b.earliest_walk_server_ms - self.server_ms) as i32;
          }
      }
      d
  } else {
      100
  };
  self.enqueue_creature_wait(cid, delay as u64);
  ```
  The cleanest approach is to resolve the source thing once in `enqueue_player_move` (the
  `CMoveObject` handler already has `sprite_id`/`from_stack_pos`; `internal_get_thing_move`
  exists) and pass an `is_creature_container: bool` into the wait/queue logic, avoiding a second
  resolution at execute time. Update the "D9" comments at lines 572-573, 605 to reflect the port.

  **P12 (C7) — execute-time adjacency re-check:** `CheckMoveObject` calls
  `ObjectAccessible(CreatureID, Obj, 1)` (`operate.cc:424`) at **execute** time (inside `::Move`,
  not `ToDoMove`). Rust only checks actor↔target adjacency at enqueue (`object_in_range` in
  `enqueue_player_move`). Once P-B adds the 1000 ms wait, the target can walk out of range during
  the wait — 772 rejects at execute, Rust still pushes. P-B must also add an execute-time adjacency
  re-check in `player_push_creature` (or in the `CreatureAction::Move` execute arm) mirroring
  `ObjectAccessible(…, 1)`. This is a **prerequisite for P-B landing safely** — without it, the
  1000 ms wait introduces a new parity regression.

**Verify:** `cargo test` — push fires no earlier than 1000 ms after the packet; a second push
during walk cooldown waits the cooldown remainder + 1000 ms; **target walks out of range during
the wait → push rejected at execute** (P12). Compare beat timing vs a `chase_kite_sim`-style
harness if available.

### Phase P-C — Destination validation (P3, P5, P6, P8, P9) [High]

**Goal:** port the `CheckMapDestination` creature-container arm (`operate.cc:493-532`) into
`player_push_creature` as a dedicated `check_push_destination` helper.

**P5 prerequisite (C1):** `Elevation` is not parsed anywhere in `crates/` (0 matches). Before the
height gate can land, the OTB `Elevation` attribute must be added to the item model and a
`tile_elevation_sum(pos) -> i32` helper added that mirrors `GetHeight` (`info.cc:689`) — sum
`ELEVATION` of all `HEIGHT`-flagged items on the tile. **Do not reuse `tile_has_height_n`** — it is
a hasHeight count, not an elevation sum.

**Files:**
- `game_world_player_throw.rs` — extract a `check_push_destination(actor, moving, from, to)`
  helper (Code Hygiene: name for its contract — "push dest validation for another creature").
  **C5:** the `AVOID` and `PZ → non-PZ` checks must stay nested inside the
  `if (OrigZ == DestZ || Type != MONSTER)` guard, **after** `MovePossible` succeeds — they only
  run when `MovePossible` was called. The helper must take `moving_type` and `dz` and mirror the
  decompile nesting, or it will reject pushes that 772 accepts (e.g. a monster changing floors
  where `MovePossible` is skipped). The `MovePossible` call itself is P-D; P-C lands the range,
  height, `ThrowPossible` checks and the *structure* of the `AVOID`/PZ nesting (the actual
  `AVOID`/PZ branches fire only after P-D's `MovePossible` returns `Ok`):
  ```rust
  /// 772 `CheckMapDestination` creature-container arm (`operate.cc:493-532`).
  /// Pre: `moving != actor`, both on map tiles, `to` is a map tile.
  /// `moving_is_monster` selects the `OrigZ == DestZ || Type != MONSTER` guard.
  fn check_push_destination(&self, actor, moving, moving_is_monster: bool, from, to) -> Result<(), ReturnValue> {
      // P3: 1-tile range.
      let dx = (to.x as i32 - from.x as i32).abs();
      let dy = (to.y as i32 - from.y as i32).abs();
      let dz = (to.z as i32 - from.z as i32).abs();
      if dx > 1 || dy > 1 || dz > 1 { return Err(ReturnValue::NotPossible); } // OUTOFRANGE
      // P5 (C1): up/down elevation-sum split. NOT tile_has_height_n — use tile_elevation_sum.
      if to.z == from.z - 1 {
          if self.tile_elevation_sum(from) < 24 { return Err(ReturnValue::NotEnoughRoom); } // NOROOM
      } else if to.z == from.z + 1 {
          if self.tile_elevation_sum(to) < 24 { return Err(ReturnValue::NotPossible); } // NOWAY
      }
      // P-D's MovePossible call goes here (Phase P-D). The AVOID/PZ block below
      // is nested inside `if (dz == 0 || !moving_is_monster)` and only runs
      // after MovePossible returns Ok — mirror the decompile (C5).
      if dz == 0 || !moving_is_monster {
          // ... P-D MovePossible call here; on Err, return its error ...
          // P6: dest AVOID (magic field) blocks pushing another creature.
          if self.tile_has_avoid(to) { return Err(ReturnValue::NotEnoughRoom); } // NOROOM
          // PZ → non-PZ (already present at lines 158-160 — move into helper).
          if self.tile_in_protection_zone(from) && !self.tile_in_protection_zone(to) {
              return Err(ReturnValue::NotPossible); // PROTECTIONZONE
          }
      }
      // P9 (C6): ThrowPossible — NOT trivially true; tests UNTHROW on dest + floor-descent loop.
      if !self.map.throw_possible(from, to, 1) { return Err(ReturnValue::CannotThrow); }
      Ok(())
  }
  ```
  Replace the inline height/PZ checks in `player_push_creature` (lines 146-160) with a call to
  this helper. Add `tile_has_avoid` (or reuse the `tilestate::MAGICFIELD` check from
  `walk_tile.rs:660`). Add `tile_elevation_sum(pos) -> i32` (new — see P5 prerequisite above).
  **Do not add a `tile_has_height_n(pos, n)` wrapper** — it is the wrong metric for this gate.

**Verify:** `cargo test` — push rejected at 2-tile range; push up from a low-elevation-sum origin
rejected (elevation 23 vs 24); push down to a low-elevation-sum dest rejected; same-floor push
ignores elevation entirely; push onto a `UNTHROW` dest tile rejected (P9/C6); push onto a
magic-field tile rejected; PZ→non-PZ rejected (regression).

### Phase P-D — Per-creature `MovePossible` (P4) [High]

**Goal:** enforce the moving creature's own `MovePossible` constraints when pushed. This is the
largest phase; it requires per-type predicates that mirror the decompile but are written as
idiomatic Rust (no vtable, no OOP).

**C2/C3 — signatures must be `Result<bool, ReturnValue>`, not `-> bool`:** `TPlayer::MovePossible`
(`crplayer.cc:366-376`) **throws** `ENTERPROTECTIONZONE` / `NOTINVITED`; it does not return `false`
for those cases. The monster kick loop **throws** `EXHAUSTED` (`crnonpl.cc:2237` player blocker,
`:2240` failed `KickCreature`) and sets `this->Target = 0` before throwing. A `-> bool` mapping
cannot express these — they propagate out of `CheckMapDestination` unchanged and produce different
client messages than `NOROOM`. All P-D predicates must return `Result<bool, ReturnValue>` where
`Err(rv)` carries the thrown result and `Ok(false)` carries the plain `false` returns (which become
`NOROOM` for pushing another creature, `MOVENOTPOSSIBLE` for self-walk).

**Files:**
- New `creature/move_possible.rs` (or extend `walk/walk_tile.rs`) with:
  - `fn player_move_possible_push(&self, cid, dest, jump) -> Result<bool, ReturnValue>` —
    `crplayer.cc:363` (base `BANK&&!UNPASS`/`JumpPossible` + PZ-enter + house-invite). The PZ-enter
    and house-invite gates already exist for self-walk; reuse them. **C2:** the PZ-enter gate throws
    `ENTERPROTECTIONZONE` (`EarliestProtectionZoneRound > RoundNr && IsProtectionZone(dest) &&
    !IsProtectionZone(origin)`) and the house-invite gate throws `NOTINVITED` (`HouseID != 0 &&
    !IsInvited && !CheckRight(ENTER_HOUSES)`) — these must be `Err(...)`, not `Ok(false)`.
  - `fn monster_move_possible_push(&self, cid, dest) -> Result<bool, ReturnValue>` —
    `crnonpl.cc:2141-2292` (same-z, home-range, `Radius`, `GO_STRENGTH` (= `base.speed < 0`,
    `crnonpl.cc:2162`), `!PZ`, `!House`, summon anti-crowd rule, **and the kick loop**). The
    decompile calls `MovePossible(Execute=true)` at `operate.cc:516` with no "is this a push?"
    branch — the full function runs, including the kick loop at `crnonpl.cc:2185-2288`. **Do NOT
    skip the kick loop** (V1 correction): it both validates the destination tile (`BANK` first-object
    at line 2187, `UNPASS` at line 2249, `AVOID` at line 2262) and performs kicks (`KickCreature`
    line 2241, `KickBoxes` line 2255/2274) with retry. Reuse the existing `monster_push.rs` kick
    logic (already implements `KickCreature`/`KickBoxes` for self-walk from `on_walk`). Skipping the
    loop would let pushes land on blocked tiles that 772 rejects, and lose the kick side effects.
    **C3:** the kick loop throws `EXHAUSTED` (`crnonpl.cc:2237` player blocker, `:2240` failed
    `KickCreature`) and sets `this->Target = 0` before throwing — this must be `Err(EXHAUSTED)` with
    the `Target = 0` side effect applied, not `Ok(false)`.
  - `fn npc_move_possible_push(&self, cid, dest) -> Result<bool, ReturnValue>` — `crnonpl.cc:1672`
    (`BANK && !UNPASS && !AVOID && z==startz && within Radius && !House`). Pure boolean — no throws
    — so `Ok(bool)` suffices but keep the `Result` signature for uniformity.
  - `fn base_move_possible(&self, dest, jump) -> Result<bool, ReturnValue>` — `crmain.cc:883`
    (`JumpPossible` or `BANK&&!UNPASS`; `!Execute && AVOID` → false — but here `Execute=true`, so
    AVOID is handled by Gate C).
- `game_world_player_throw.rs` — after `check_push_destination`, call the moving creature's
  `MovePossible`:
  ```rust
  let result = match self.creatures.get(moving_creature) {
      Some(CreatureKind::Player(_)) => self.player_move_possible_push(moving_creature, to_pos, from.z != to.z),
      Some(CreatureKind::Monster(_)) => self.monster_move_possible_push(moving_creature, to_pos),
      Some(CreatureKind::Npc(_)) => self.npc_move_possible_push(moving_creature, to_pos),
      None => Ok(false),
  };
  match result {
      // C2/C3: thrown results propagate unchanged (ENTERPROTECTIONZONE, NOTINVITED, EXHAUSTED).
      Err(rv) => return Err(rv),
      Ok(false) => return Err(ReturnValue::NotEnoughRoom), // NOROOM for pushing another creature
      Ok(true) => {}
  }
  ```
  Cite `operate.cc:515-522` (the `CreatureID != MovingCreature->ID` → `NOROOM` arm for `Ok(false)`;
  thrown results are not caught and remapped).

**Note on `GO_STRENGTH` (Q1 — resolved):** the 772 `Skills[SKILL_GO_STRENGTH]->Act < 0` check
(`crnonpl.cc:2162`) gates whether the monster is allowed to move at all. In this server,
`GO_STRENGTH` maps to the creature's `speed` attribute (`CreatureBase::speed`, `creature/base.rs:193`
— "tracks vocation GoStrength"). The check is therefore `base.speed < 0`. P-D's
`monster_move_possible_push` should test `base.speed < 0` → `Ok(false)`. NPCs have a dedicated
`NpcMovement::go_strength` field (`npcs/mod.rs:69`); monsters use `base.speed`.

**Note on summon anti-crowd (C4 — resolved):** `crnonpl.cc:2171-2181` is **not** a leash. It rejects
when `Execute && Master != 0 && State ∉ {ATTACKING, PANIC}` and master is on the same z and the
summon is currently **not** adjacent (manhattan > 1) and the destination **would be** adjacent
(≤ 1). Implement the anti-crowd predicate exactly; do not add a distance cap.

**Verify:** `cargo test` — push a monster out of its home range → `NOROOM`; push a monster into a
house → `NOROOM`; push a monster into a PZ → `NOROOM`; push an NPC outside its radius → `NOROOM`;
**push a player into a PZ during `EarliestProtectionZoneRound` → `ENTERPROTECTIONZONE` message, not
`NOROOM` (C2)**; **push a player into an uninvited house → `NOTINVITED` message, not `NOROOM`
(C2)**; **push a monster onto a player-blocker tile → `EXHAUSTED` + `Target = 0` side effect, not
`NOROOM` (C3)**; **push a summon currently 2 tiles from master onto a tile adjacent to master →
rejected (C4 anti-crowd, not leash)**.

### Phase P-E — Cleanup + lessons [Low]

- Remove the now-dead `is_pushable()` call site in `player_push_creature` (keep the fn for
  monster AI).
- Update the "D9" markers in `creature_todo.rs` to "ported".
- Add C++ reference headers per `TFS-cpp-references.md` to any new functions in
  `creature/move_possible.rs`.
- Update `tasks/lessons.md` with:
  - 772 pushability is race-driven (`Unpushable`), not TFS `pushable && speed != 0`.
  - NON_PVP peaceful-summon exception.
  - `ToDoMove` creature-container delay = 1000 ms + walk cooldown (not 100 ms).
  - Up-floor height check uses **origin** elevation sum; down-floor uses **dest** elevation sum.
  - `MovePossible` is called on the **moving** creature in `CheckMapDestination`, not the actor.
  - **V1:** `MovePossible(Execute=true)` runs the **full** function during pushes, including the
    kick loop (`crnonpl.cc:2185-2288`) — do not skip it. Reuse `monster_push.rs`.
  - **V2:** `Radius` is a per-creature spawn-wave field (`crmain.cc:174`, default `INT_MAX`; also
    `crmain.cc:1760`), not a race flag. Rust `Monster` doesn't model it — add the field or default
    to `i32::MAX`.
  - **V3:** TFS data pack conflates 772's independent `unpushable`/`kickcreatures` flags
    (`canPushCreatures → pushable=false`). A 772-accurate monster with `kickcreatures` but pushable
    cannot be expressed in the TFS data pack.
  - **Q1:** `GO_STRENGTH` maps to `CreatureBase::speed` (`creature/base.rs:193`); the 772
    `Act < 0` check (`crnonpl.cc:2162`) is `base.speed < 0`. NPCs use `NpcMovement::go_strength`.
  - **C1:** 772 `GetHeight` (`info.cc:689`) is an **elevation sum** (`ELEVATION` attr of `HEIGHT`
    items), not a hasHeight count. `tile_has_height_n` is the wrong metric. `Elevation` is not
    parsed in `crates/` — must be added before the height gate can land. The existing height check
    in `player_push_creature` is dead code (count ≥ 24 never true, no `dz` condition).
  - **C2:** `TPlayer::MovePossible` **throws** `ENTERPROTECTIONZONE`/`NOTINVITED` — P-D predicates
    must be `Result<bool, ReturnValue>`, not `-> bool`, or the client gets `NOROOM` instead of the
    correct message.
  - **C3:** The monster kick loop **throws** `EXHAUSTED` (`crnonpl.cc:2237`,`:2240`) and sets
    `Target = 0` before throwing — same `Result` requirement.
  - **C4:** `crnonpl.cc:2171-2181` is an **anti-crowd** rule (summon not-adjacent → dest-adjacent
    rejected), not a leash/distance cap. Do not implement a distance cap.
  - **C5:** `AVOID` and `PZ → non-PZ` checks are nested inside `if(OrigZ == DestZ || Type !=
    MONSTER)` after `MovePossible` — the helper must mirror this nesting.
  - **C6:** `ThrowPossible(…, 1)` is **not** trivially true — it tests `UNTHROW` on the dest tile
    and runs a floor-descent loop for `dz != 0`. P9 is Medium, not Low.
  - **C7:** Three findings the first pass missed — `CheckTopMoveObject` (P11, only first creature
    in tile stack is pushable), execute-time `ObjectAccessible` (P12, gets worse with P-B's wait),
    and `SeparationEvent`/`MovementEvent`/`CollisionEvent` for pushed creatures (P13, unverified
    coverage).
  - **Minor:** P-A changes the unpushable message from `NotPossible` ("Sorry, not possible.") to
    `NotMoveable` ("You cannot move this object.") — correct per 772, but an observable change.
  - **Minor:** `home_position`/`radius` are on `NpcRuntimeState` (`creature/npc.rs:54-55`), not
    `Npc`. Race flag assignment is `crmain.cc:1496` (`else if` on 1495).

---

## 5. Verification (per phase)

| Phase | `cargo` | Tests |
|---|---|---|
| P-A | `rtk cargo check`, `rtk cargo clippy`, `rtk cargo test` | NON_PvP peaceful-summon pushable; PVP unpushable-race blocked; normal monster pushable; **unpushable message is `NotMoveable` not `NotPossible` (minor nit)** |
| P-B | same | Push delay ≥ 1000 ms; walk-cooldown remainder added; **target walks out of range during wait → rejected at execute (P12)** |
| P-C | same | 2-tile push rejected; **up-from-low-elevation-sum origin rejected (23 vs 24)**; **down-to-low-elevation-sum dest rejected**; **same-floor push ignores elevation**; **`UNTHROW` dest tile rejected (P9/C6)**; magic-field dest rejected; PZ boundary regression |
| P-D | same | Out-of-home-range monster → `NOROOM`; house/PZ monster → `NOROOM`; NPC radius → `NOROOM`; **player into PZ during `EarliestProtectionZoneRound` → `ENTERPROTECTIONZONE` (C2)**; **player into uninvited house → `NOTINVITED` (C2)**; **monster onto player-blocker → `EXHAUSTED` + `Target=0` (C3)**; **summon 2-from-master pushed to master-adjacent → rejected (C4 anti-crowd)**; push onto blocked tile triggers kick (or rejects if kick fails) |
| P-E | same + `rtk cargo clippy -- -D warnings` | No new dead code; lessons updated; **`Elevation` parsing verified (`rtk grep -n ELEVATION crates/`)** |

All phases: confirm 1098 regression — the push path is shared (`player_move_thing` is era-agnostic
via the ToDo engine). The 772-specific gates (NON_PVP exception, `MovePossible` constraints) must
not break 1098; if a 1098 profile diverges, put the era knob in `MechanicsProfile` / formulas Lua,
not a parallel `_772.rs` file.

---

## 6. Open questions (all resolved 2026-08-08 re-verification)

1. **`GO_STRENGTH`** — **Modeled as `speed`.** 772 `Skills[SKILL_GO_STRENGTH]->Act` maps to
   `CreatureBase::speed` (`creature/base.rs:193`). The `Act < 0` check (`crnonpl.cc:2162`) is
   `base.speed < 0`. P-D tests this directly. NPCs have a separate `NpcMovement::go_strength`
   (`npcs/mod.rs:69`).
2. **`EarliestProtectionZoneRound`** — **Modeled.** `Player::earliest_protection_zone_round: u32`
   (`creature/player.rs:364`). Already used in `walk_tile.rs:585-591` for self-walk PZ-enter gate.
   Reuse for P-D player arm.
3. **`IsInvited` / house-invite** — **Modeled.** `House::is_invited(house_id, player_guid)`
   (`house.rs:154`). House tiles checked via `matches!(tile, Tile::House(_))`. Reuse for P-D player
   arm.
4. **NPC `startz`/`Radius`** — **Modeled.** `NpcRuntimeState::home_position: Position` and
   `NpcRuntimeState::radius: u16` (`creature/npc.rs:54-55`, on `NpcRuntimeState` not `Npc` — minor
   nit from first pass). Available for P-D NPC arm.

---

## 7. Second-pass corrections (2026-08-08)

Seven corrections applied after a second source pass over
`reference/cipsoft-772/tibia-game-master/src/` and the Rust tree. Each is reflected in the sections
above; this section is the canonical index.

| # | Correction | Severity | Where fixed |
|---|------------|----------|-------------|
| **C1** | **P5 is materially wrong, and the P-C fix would perpetuate it.** 772 `GetHeight` (`info.cc:689`) sums the `ELEVATION` **attribute** of `HEIGHT`-flagged objects; the gate is `sum < 24`. Rust `tile_has_height_n` (`walk/walk_tile.rs:39`) is TFS `Tile::hasHeight(n)` — a **count** of hasHeight items, true at `count >= n`. So the existing check is (a) the wrong metric, (b) inverted (`count >= 24` is never true → dead code), and (c) has **no `dz` condition** — it nominally applies to same-floor pushes too. **Every floor-change push passes today.** P-C reuses `tile_has_height_n` and inherits all three bugs. Prerequisite the audit omits: `Elevation` is **not parsed anywhere in `crates/`** (0 matches) — the OTB `Elevation` attribute must be added before this gate can be ported. P5 upgraded Medium → **High**. | **High** | §0 P5, §2 Gate C, §3.1 #4, §3.3, P-C |
| **C2** | **P-D player arm signature is wrong.** `TPlayer::MovePossible` (`crplayer.cc:366-376`) **throws** `ENTERPROTECTIONZONE` / `NOTINVITED`; it does not return `false`. Those propagate out of `CheckMapDestination` unchanged — they are *not* remapped to `NOROOM`. The proposed `-> bool` + `Err(NotEnoughRoom)` mapping produces the wrong client message. P-D predicates must be `Result<bool, ReturnValue>`. | **High** | §0 P4, P-D |
| **C3** | **P-D monster arm has the same defect.** The kick loop throws `EXHAUSTED` (`crnonpl.cc:2237` player blocker, `:2240` failed `KickCreature`), and sets `this->Target = 0` before throwing. `-> bool` cannot express this. Both arms need `Result<bool, ReturnValue>`. | **High** | §0 P4, P-D |
| **C4** | **"summon-leash" is a mischaracterization.** `crnonpl.cc:2171-2181` is the inverse of a leash: given `Execute && Master != 0 && State ∉ {ATTACKING, PANIC}` and master on the same z, it rejects when the summon is currently **not** adjacent (manhattan > 1) and the destination **would be** adjacent (≤ 1) — an anti-crowding rule. Implementing "leash" (distance cap) would be wrong. | Medium | §0 P4, §2 Gate B, P-D |
| **C5** | **P-C hoists checks out of their guard.** The `AVOID` and PZ checks are nested **inside** `if(OrigZ == DestZ \|\| Type != MONSTER)`, **after** `MovePossible` succeeds. The proposed `check_push_destination` runs them unconditionally and before `MovePossible`. §2 notes the structural difference but the plan dropped it. The helper must mirror the nesting. | Low | §2 Gate C, P-C |
| **C6** | **P9 is not "trivially true".** `ThrowPossible` (`info.cc`) iterates `T = 1..MaxT`, so at range 1 it *does* test `UNTHROW` on the destination tile, plus the floor-descent loop for `dz != 0`. It is a real gate, not a formality. P9 upgraded Low → **Medium**. | Medium | §0 P9, §2 Gate C, §3.1 #12, P-C |
| **C7** | **Three findings missing entirely.** (a) `CheckTopMoveObject` (`operate.cc:1356`) throws `NOTACCESSIBLE` when the pushed creature isn't the top move object — only the *first* creature in the tile stack is eligible. Not implemented, not listed → **P11**. (b) `CheckMoveObject`'s `ObjectAccessible(CreatureID, Obj, 1)` (`operate.cc:424`) re-checks actor↔target adjacency at **execute** time. Rust only checks it at enqueue. This regression *gets worse* once P-B adds the 1000 ms wait — the target can walk away and 772 rejects while Rust still pushes. Belongs in P-B → **P12**. (c) `SeparationEvent(Obj, OldCon)` fires before `MoveObject` (`operate.cc:1386`) and `MovementEvent`/`CollisionEvent` after, for creature pushes too. §1.4 lists them but §3.1 never audits whether `flush_pending_creature_step_events` / `apply_notify_go_after_relocate` cover the push path (trap/teleporter/field on the destination) → **P13**. | Medium | §0 P11-P13, §3.1 #13-15, P-B |

**Minor nits (not separate findings):**
- `home_position`/`radius` are on `NpcRuntimeState`, not `Npc` (§6 #4 fixed).
- Race-flag assignment is `crmain.cc:1496` (`else if` on 1495); `Radius = INT_MAX` is also set at
  `crmain.cc:1760` (P-E lessons fixed).
- P-A silently changes the Gate A return from `NotPossible` to `NotMoveable` — correct per 772, but
  an observable message change (P-A note added).
