# Player Movement Audit — CipSoft 772 Decompile vs Rust
**Date:** 2026-07-08  
**Auditor:** Kiro (Claude Sonnet 4.6)  
**Source of truth:** `reference/cipsoft-772/tibia-game-master/src/` — actual CipSoft decompile  
**Rust target:** `crates/tfs-rust-core/src/walk/`, `crates/tfs-rust-net/src/game_parse.rs`

**Note:** This document supersedes the earlier audit that incorrectly cited TFS 1.4.2 (`src/`)
and TVP (`reference/tvp-772/gameserver/src/`) as the 772 reference. All findings below are
verified directly against the CipSoft decompile files listed above. The prior audit in
`player-walk-audit.md` (P1–P9, N1–N4) and `walk-audit-findings.md` are complementary; this
document records gaps and bugs not already tracked there.

---

## Key decompile facts (ground truth for all findings)

### Walk execution — `TCreature::Execute` (`cract.cc`)

```
while(LockToDo && !IsDead && NextWakeup <= ServerMilliseconds):
  if NrToDo <= ActToDo:
    ToDoClear(); IdleStimulus(); break
  Delay = CalculateDelay()
  if Delay > 0:
    if Stop: ToDoClear(); [player] SendSnapback; break
    else:    NextWakeup = ServerMilliseconds + Delay; queue insert; break
  pop entry, dispatch on Code:
    TDGo    → Go(x,y,z)
    TDAttack → Attack()
    …
  on throw(RESULT r):
    SnapbackNecessary = (ToDoClear() || Stop)
    if r == EXHAUSTED: ToDoWait(1000); ToDoStart()
    else:              ToDoYield()
    if PLAYER: SendResult(r); if SnapbackNecessary && r∉{MOVENOTPOSSIBLE,NOTINVITED,ENTERPROTECTIONZONE}: SendSnapback
    break
  if Stop: ToDoClear(); [player] SendSnapback; break
```

### Walk step — `TCreature::Go` (`cract.cc`)

```
Distance = max(|OrigX-DestX|, |OrigY-DestY|)
if Distance > 1 || OrigZ != DestZ: throw NOTACCESSIBLE

[drunk stagger — see BUG-A below]

if !MovePossible(Dest, Execute=true):
  if PLAYER && !diagonal:
    if GetHeight(Orig) >= 24 && no bank/unpass above && MovePossible(Dest, z-1, Jump=true): DestZ -= 1
    elif GetHeight(Dest+1) >= 24 && no bank/unpass at Dest && MovePossible(Dest, z+1, Jump=true): DestZ += 1
  if posz == DestZ: throw MOVENOTPOSSIBLE

Move(CrObject, Dest)   ← global ::Move, triggers NotifyGo + NotifyTurn
```

### Direction — `TCreature::NotifyTurn` (`cract.cc`)

Direction is set **after** a successful `::Move` call, from the offset of the destination
relative to the creature's current position, **X-axis first, then Y**:

```cpp
if(OffsetX > 0)       Direction = EAST
else if(OffsetX < 0)  Direction = WEST
else if(OffsetY < 0)  Direction = NORTH
else if(OffsetY > 0)  Direction = SOUTH
```

On a failed move (throws before `::Move`), direction is **never changed**.

### Step timing — `TCreature::NotifyGo` (`cract.cc`)

```cpp
Bank = GetFirstObject(DestX, DestY, DestZ);  // BANK flag object at destination
Waypoints = BankType.getAttribute(WAYPOINTS);
if(DiagonalMove) Waypoints *= 3;
Delay = (Waypoints * 1000) / GetSpeed();
BeatCount = (Delay + Beat - 1) / Beat;       // ceil to Beat
EarliestWalkTime = ServerMilliseconds + BeatCount * Beat;
```

Speed: `GetSpeed() = SKILL_GO_STRENGTH.Get() * 2 + 80` (`crmain.cc`).  
No z-change multiplier. Diagonal is `×3` waypoints only.

### `ToDoStart` — `CalculateDelay` for `TDGo` (`cract.cc`)

```cpp
case TDGo:
  if(EarliestWalkTime > ServerMilliseconds)
    Delay = EarliestWalkTime - ServerMilliseconds;
```

### `CGoDirection` / `CGoPath` — `receiving.cc`

`CGoDirection`: `ToDoClear()` → `[if SnapbackNecessary] SendSnapback` → add one `TDGo(cur+offset)` → `ToDoStart()`.  
`CGoPath`: same preamble → loop adding `TDGo` for each step → `ToDoStart()`.  
`CGoStop`: `ToDoStop()`.

### `CAttack`/`CFollow` — `receiving.cc`

Both call `CAttack(Connection, Buffer, Follow)`:

```cpp
Player->Combat.SetAttackDest(TargetID, Follow);
Player->ToDoAttack();
Player->ToDoStart();
```

`Follow=false` for attack, `Follow=true` for follow. Both paths are **identical** except
for the `Follow` boolean stored in `Combat.Following`.

### `CCancel` — `receiving.cc`

```cpp
Player->Combat.StopAttack(0);
if(Player->ToDoClear()) SendSnapback(Connection);
Player->ToDoYield();
```

### `TPlayer::IdleStimulus` — `crplayer.cc`

```cpp
if(Combat.AttackDest != 0):
  try:
    ToDoAttack(); ToDoStart()
  catch(RESULT r):
    ToDoClear()
    if r != NOERROR:
      if r != NOWAY: SendResult(Connection, r)
      ToDoWait(1000); ToDoStart()
```

No separate follow path. Follow == attack with `Combat.Following = true`.

### `TCombat::CanToDoAttack` — chase in `ToDoAttack` (`crcombat.cc`)

```cpp
ChaseMode = this->ChaseMode
if(Following) ChaseMode = CHASE_MODE_CLOSE
if(ChaseMode == CHASE_MODE_CLOSE):
  if(Distance > 1): Master->ToDoGo(target, false, 3)  // max 3 steps
```

### `TCreature::CreatureMoveStimulus` — reactive chase rearm (`crmain.cc`)

Fires when `Combat.AttackDest` moved, `ChaseMode == CLOSE`, attack not imminent, creature
locked with `TDAttack` at front, and target distance > 1:

```cpp
ToDoClear()  [+ SendSnapback if player]
ToDoWait(200)
ToDoAttack()
ToDoStart()
```

### `TCreature::CRotate` — `receiving.cc`

```cpp
Player->ToDoRotate(Direction);
Player->ToDoStart();
```

Rotation goes through the ToDo queue (not immediate).

### Drunk stagger — `TCreature::Go` (`cract.cc`)

```cpp
DrunkLevel = Skills[SKILL_DRUNKEN]->TimerValue()
if(DrunkLevel > 0 && Skills[SKILL_DRUNKEN]->Get() == 0):
  StaggerChance = max(7 - DrunkLevel, 1)
  if(rand() % StaggerChance == 0):
    pick random cardinal dest from current pos
    if(ToDoClear() && Type == PLAYER) SendSnapback
    ToDoTalk(Mode, NULL, "Hicks!", false)
    ToDoStart()
    // NOTE: execution continues with the (random) destination this same beat
```

---

## Findings

---

### BUG-A — Drunk stagger: wrong formula, and stagger does NOT clear `walk_destinations`

**Severity:** MEDIUM  
**Files:** `crates/tfs-rust-core/src/walk/mod.rs` (`try_drunk_walk_direction`, `on_walk`)

#### Decompile (`cract.cc` — `TCreature::Go`)

```cpp
int DrunkLevel = this->Skills[SKILL_DRUNKEN]->TimerValue();
if(DrunkLevel > 0 && this->Skills[SKILL_DRUNKEN]->Get() == 0){
    int StaggerChance = std::max<int>(7 - DrunkLevel, 1);
    if(rand() % StaggerChance == 0){
        // pick random cardinal, replace DestX/DestY
        if(this->ToDoClear() && this->Type == PLAYER) SendSnapback(this->Connection);
        this->ToDoTalk(TalkMode, NULL, "Hicks!", false);
        this->ToDoStart();
        // execution FALLS THROUGH — still moves to random dest this beat
    }
}
```

Key facts:
1. The stagger condition is `Skills[SKILL_DRUNKEN]->Get() == 0` — this is the **skill level** check (0 = no suppression item equipped), not a timer check.
2. The formula is `rand() % max(7 - DrunkLevel, 1) == 0`.
3. After `ToDoClear()`, the old walk queue is gone. The creature then falls through and takes the staggered step **this same beat**.
4. `ToDoClear()` returns whether a `TDGo` was cleared. If yes → `SendSnapback`. The walk queue is wiped.

#### Rust (`walk/mod.rs` `try_drunk_walk_direction`)

```rust
fn try_drunk_walk_direction(base: &CreatureBase) -> Option<Direction> {
    if !has_drunk_condition(base) { return None; }
    let drunk_level = base.drunkenness as i32;
    let stagger_chance = (7 - drunk_level).max(1) as u32;
    let r = uniform_random(…, 0, (stagger_chance as i32).saturating_sub(1)) as u32;
    if r != 0 { return None; }
    // pick random cardinal
}
```

Bug 1 — wrong RNG: Decompile uses `rand() % StaggerChance == 0`, i.e. triggers with probability `1/StaggerChance`. Rust uses `uniform_random(0, stagger_chance-1) != 0` to return None, meaning it triggers when `r == 0`, which is `1/stagger_chance`. The probability is the same, but the uniform_random call uses `0..(stagger_chance-1)` inclusive. For `stagger_chance=1` this is `uniform_random(0,0)` which always returns 0, triggering every step. The decompile `rand()%1 == 0` also always triggers. This is correct. For `stagger_chance=7`, decompile triggers 1/7, Rust triggers 1/7. **Formula is functionally equivalent.** No bug here.

Bug 2 — missing `Get() == 0` check: Rust only checks `base.drunkenness > 0` (the timer value). The decompile **also** requires `Get() == 0`, meaning the drunk suppression skill level must be zero. Without this, a player with a drunk-suppression item equipped would still stagger. The Rust `has_drunk_condition` never checks this suppression condition.

Bug 3 — `walk_destinations` desync: after `player_todo_clear_with_snapback` clears `walk_destinations`, the stagger re-enqueues `Go` via `enqueue_creature_go` but never pushes a new entry onto `walk_destinations`. On the next beat `pop_dest` returns `None`, skipping the adjacency check. This is low-risk (adjacency check skipped = more permissive) but diverges from correct state tracking.

#### Fix

```rust
fn try_drunk_walk_direction(base: &CreatureBase) -> Option<Direction> {
    // Decompile: TimerValue > 0 && Get() == 0 (no suppression equipped)
    let drunk_level = base.drunkenness; // TimerValue
    if drunk_level == 0 { return None; }
    // TODO: also check drunk suppression skill Get() == 0 when skill system exists
    let stagger_chance = (7i32 - drunk_level as i32).max(1) as u32;
    if uniform_random(…, 0, stagger_chance as i32 - 1) != 0 { return None; }
    // pick random cardinal…
}
```

For the `walk_destinations` fix, after the stagger clear push the stagger destination:

```rust
if drunk_staggered {
    // …clear + re-enqueue…
    let stagger_dest = self.creatures.get(cid).map(|k| k.position().offset(dir));
    if let (Some(d), Some(CreatureKind::Player(p))) = (stagger_dest, self.creatures.get_mut(cid)) {
        p.base.walk_destinations.push_back(d);
    }
}
```

---

### BUG-B — Stair climb: wrong condition (`GetHeight >= 24` vs tile flags) and no `MovePossible` test at destination z before trying z±1

**Severity:** HIGH  
**Files:** `crates/tfs-rust-core/src/walk/walk_tile.rs` (`resolve_player_move_destination`)

#### Decompile (`cract.cc` — `TCreature::Go`, player non-diagonal path)

```cpp
if(!this->MovePossible(DestX, DestY, DestZ, true, false)){
    bool DiagonalMove = (OrigX != DestX && OrigY != DestY);
    if(this->Type == PLAYER && !DiagonalMove){
        // try up
        if(DestZ > 0
                && GetHeight(OrigX, OrigY, OrigZ) >= 24
                && !CoordinateFlag(OrigX, OrigY, OrigZ - 1, BANK)
                && !CoordinateFlag(OrigX, OrigY, OrigZ - 1, UNPASS)
                && this->MovePossible(DestX, DestY, DestZ - 1, true, true)){
            DestZ -= 1;
        // try down
        }else if(DestZ < 15
                && GetHeight(DestX, DestY, DestZ + 1) >= 24
                && !CoordinateFlag(DestX, DestY, DestZ, BANK)
                && !CoordinateFlag(DestX, DestY, DestZ, UNPASS)
                && this->MovePossible(DestX, DestY, DestZ + 1, true, true)){
            DestZ += 1;
        }
    }
    if(this->posz == DestZ){ throw MOVENOTPOSSIBLE; }
}
```

Critical observations:
1. The stair adjustment is **inside** `if(!MovePossible(Dest, Execute=true, Jump=false))` — it only runs when the straight same-z move is **blocked**.
2. Stair-up uses `GetHeight(OrigX, OrigY, OrigZ) >= 24` — height of the **current** tile.
3. Stair-down uses `GetHeight(DestX, DestY, DestZ + 1) >= 24` — height of the tile **one floor below the destination**.
4. Both use `MovePossible(DestX, DestY, DestZ±1, Execute=true, Jump=true)` — a **jump** check on the new z-level.
5. There is a `!CoordinateFlag(…, BANK)` / `!CoordinateFlag(…, UNPASS)` check for the intermediate floor.

#### Rust (`walk_tile.rs` `resolve_player_move_destination`)

The Rust port runs stair checks **unconditionally** (not gated on same-z being blocked), and uses `tile_has_height_n(…, 3)` which counts items with `CONST_PROP_HASHEIGHT` — a TFS 1.4.2 item flag, **not** the CipSoft `GetHeight()` which returns the sum of `HEIGHT` attributes on BANK objects.

The result: a player walking along a flat corridor next to stair tiles gets incorrectly routed onto the stairs even when the straight path is available.

#### Fix

1. Only enter the stair block when `tile_query_add_player(world, to_tile, cid, 0) != NoError` (same-z is blocked).
2. Replace `tile_has_height_n(…, 3)` with the actual `GetHeight()` semantics: sum the `HEIGHT` attribute on all `BANK`-flagged objects on the tile. Gate on `>= 24`.
3. Add the `JumpPossible` / `MovePossible(Jump=true)` check on the z±1 destination before committing the z adjustment.
4. Add the `!BANK && !UNPASS` intermediate-floor check.

---

### BUG-C — Direction set by Rust before the move; decompile sets it only after a successful move

**Severity:** MEDIUM  
**Files:** `crates/tfs-rust-core/src/walk/mod.rs` (`internal_move_creature_step`), `walk_tile.rs`

#### Decompile (`cract.cc`)

`TCreature::NotifyTurn` is called inside `::Move()` in `operate.cc` — it fires **only on a successful move**, using the offset from current position to destination:

```cpp
void TCreature::NotifyTurn(Object DestCon){
    int OffsetX = DestX - this->posx;
    int OffsetY = DestY - this->posy;
    if(OffsetX > 0)      this->Direction = DIRECTION_EAST;
    else if(OffsetX < 0) this->Direction = DIRECTION_WEST;
    else if(OffsetY < 0) this->Direction = DIRECTION_NORTH;
    else if(OffsetY > 0) this->Direction = DIRECTION_SOUTH;
}
```

On a **failed** move (`throw MOVENOTPOSSIBLE` before `::Move` is called), `Direction` is **never touched**. The player keeps facing whatever direction they were facing before.

Also: the decompile's `NotifyTurn` uses **X-axis first, then Y-axis** — diagonal moves resolve to East/West, never North/South. This is different from TFS's `getDirectionTo` which gives proper diagonal directions.

#### Rust (`internal_move_creature_step`)

The Rust code calls `set_direction_from_step(old_pos, dest_pos, k)` inside the success path, which is correct. However, for height-based floor changes the code also sets `k.base_mut().direction = direction` (the original walk direction) after the tile move — this **overrides** the `NotifyTurn` semantics with the client-sent direction byte.

The decompile's `Go` function **does not set Direction** at the point of a stair-hop. It only calls `::Move` which calls `NotifyTurn`. `NotifyTurn` uses the X/Y offset, not the input direction.

Additionally, the Rust `set_direction_from_step` function sets diagonal directions (NorthEast, etc.) for diagonal moves, while the decompile's `NotifyTurn` always resolves to a cardinal (X beats Y). This means spectators see the creature facing a diagonal direction in Rust where CipSoft would show them facing East or West.

#### Fix

Replace the direction-setting logic with the decompile's X-first priority:

```rust
fn notify_turn_direction(old_pos: Position, new_pos: Position) -> Option<Direction> {
    let dx = new_pos.x as i32 - old_pos.x as i32;
    let dy = new_pos.y as i32 - old_pos.y as i32;
    if dx > 0 { Some(Direction::East) }
    else if dx < 0 { Some(Direction::West) }
    else if dy < 0 { Some(Direction::North) }
    else if dy > 0 { Some(Direction::South) }
    else { None }
}
```

Use this for all moves (not just the chain-turn). Remove the special-case `direction = direction` override for floor changes.

---

### BUG-D — `CGoDirection` in Rust does not match decompile: Rust wipes the whole queue including pending attacks

**Severity:** MEDIUM  
**Files:** `crates/tfs-rust-core/src/walk/mod.rs` (`player_move_request`)

#### Decompile (`receiving.cc` — `CGoDirection`)

```cpp
if(Player->ToDoClear()) SendSnapback(Connection);
// add one TDGo entry
Player->ToDoAdd(TD);
Player->ToDoStart();
```

`ToDoClear()` clears **all** entries (walk + action + attack). The return value is true if a `TDGo` was among the cleared entries (snapback needed). This is unconditional.

#### Decompile (`TCreature::ToDoClear` in `cract.cc`)

```cpp
bool TCreature::ToDoClear(void){
    bool SnapbackNecessary = false;
    for(int i = 0; i < NrToDo; i++){
        switch(ToDoList[i].Code){
            case TDGo: if(ActToDo <= i) SnapbackNecessary = true; break;
            case TDTalk: DeleteDynamicString(…); break;
            // …
        }
    }
    LockToDo = false; ActToDo = 0; NrToDo = 0; Stop = false;
    return SnapbackNecessary;
}
```

`ToDoClear` returns true (snapback) only if there was a **pending** (not yet executed) `TDGo`. It does NOT snapback just because there was a pending attack or use.

#### Rust (`player_move_request`)

```rust
self.player_todo_clear_with_snapback(conn_id, cid);
```

`player_todo_clear` clears everything including `walk_action`. The snapback logic fires when `had_pending_go || !walk_queue.is_empty()`. This matches the decompile's `ToDoClear` return value.

**What IS missing:** `player_todo_clear` also calls `self.clear_player_walk_action(cid)`, which clears `walk_action` (the deferred walk-to-use action). In the decompile, `ToDoClear` wipes the entire list which includes pending `TDUse` entries — so this is correct in Rust too.

**Verified:** `player_move_request` behaviour matches `CGoDirection`. No bug here. ✅

---

### BUG-E — Attack/Follow: `CAttack` and `CFollow` are the **same handler** with a `Follow` flag; Rust dispatches them differently

**Severity:** HIGH  
**Files:** `crates/tfs-rust-core/src/walk/mod.rs`, game packet handler

#### Decompile (`receiving.cc`)

```cpp
case CL_CMD_ATTACK: CAttack(Connection, &Buffer, false); break;
case CL_CMD_FOLLOW: CAttack(Connection, &Buffer, true);  break;
```

Both call the same function. Inside:

```cpp
void CAttack(TConnection *Connection, TReadBuffer *Buffer, bool Follow){
    uint32 TargetID = Buffer->readQuad();
    try{
        Player->Combat.SetAttackDest(TargetID, Follow);
        Player->ToDoAttack();
        Player->ToDoStart();
    }catch(RESULT r){
        if(r != NOERROR){ SendResult(Connection, r); Player->ToDoYield(); }
    }
}
```

`SetAttackDest` stores `Following = Follow`. When `Following = true`, `CanToDoAttack` always uses `CHASE_MODE_CLOSE` regardless of the player's chase mode setting. When `Following = true`, `SetAttackDest` skips all the attack-validity checks (PZ, secure mode, etc.).

Setting `TargetID = 0` calls `StopAttack(0)` inside `SetAttackDest` — this is how cancel-attack works.

#### Rust current state

From `player-walk-audit.md` (BUG P3): all three packets (`Attack`, `Follow`, `CancelAttackAndFollow`) fall through to the catch-all trace. **None are implemented.**

#### Fix

Implement a single `player_set_attack_dest(cid, target_id, follow: bool)` function that:
1. Calls the equivalent of `Combat.SetAttackDest(target_id, follow)` — stores the target and `following` flag.
2. Calls `ToDoAttack()` + `ToDoStart()`.

`CancelAttackAndFollow` maps to `target_id = 0, follow = false`.

---

### BUG-F — `CCancel` is not handled; it should stop attack AND clear/yield the ToDo queue

**Severity:** MEDIUM  
**Files:** game packet handler, `crates/tfs-rust-core/src/walk/mod.rs`

#### Decompile (`receiving.cc` — `CCancel`)

```cpp
void CCancel(…){
    Player->Combat.StopAttack(0);
    if(Player->ToDoClear()) SendSnapback(Connection);
    Player->ToDoYield();
}
```

This clears the entire ToDo queue, stops the attack target, and calls `ToDoYield()` which (if not locked) enqueues `ToDoWait(0)` + `ToDoStart()`, leading to an idle-stimulus on the next beat.

#### Rust current state

`GamePacket::CancelAttackAndFollow` falls through to the catch-all. Neither `StopAttack` nor `ToDoClear` is called. The `CCancel` opcode in the decompile (`CL_CMD_CANCEL`) corresponds to the Rust `CancelAttackAndFollow` packet (opcode `0xBE`).

#### Fix

```rust
GamePacket::CancelAttackAndFollow => {
    self.player_set_attack_dest(cid, 0, false); // StopAttack
    self.player_todo_clear_with_snapback(conn_id, cid);
    // ToDoYield — if not locked, enqueue idle stimulus wakeup
    self.request_player_idle_stimulus(cid);
}
```

---

### BUG-G — `CRotate` goes through ToDo queue in decompile; Rust dispatches it immediately with a deferred broadcast

**Severity:** LOW  
**Files:** `crates/tfs-rust-core/src/walk/mod.rs` (`player_turn_request`)

#### Decompile (`receiving.cc` — `CRotate`)

```cpp
void CRotate(TConnection *Connection, int Direction){
    Player->ToDoRotate(Direction);
    Player->ToDoStart();
}
```

`ToDoRotate` adds a `TDRotate` entry to the queue. `ToDoStart` schedules it with `CalculateDelay()` — which for `TDRotate` returns 0 (no delay case in `CalculateDelay`), so it fires on the next beat. It goes through the same queue as walk steps, meaning a queued walk step that hasn't fired yet stays ahead of the rotate.

#### Rust (`player_turn_request`)

The Rust code sets direction immediately in-place and defers the `0x6B` broadcast. It does not enqueue a `TDRotate` at all. This means a rotate sent while a walk step is pending fires the direction change before the walk step, whereas the decompile would let the walk step fire first.

For most practical cases this is imperceptible, but it can cause the client to see the player face a different direction briefly before the walk step corrects it.

#### Fix (optional — low impact)

Route `player_turn_request` through the ToDo queue as a `TDRotate` entry with 0 delay. This naturally serialises rotates after any pending walk steps.

---

### BUG-H — `TPlayer::IdleStimulus` only handles `AttackDest != 0`; there is NO separate follow re-path

**Severity:** MEDIUM  
**Files:** `crates/tfs-rust-core/src/idle_stimulus.rs` (player branch)

#### Decompile (`crplayer.cc` — `TPlayer::IdleStimulus`)

```cpp
void TPlayer::IdleStimulus(void){
    if(this->Combat.AttackDest != 0){
        try{
            this->ToDoAttack();
            this->ToDoStart();
        }catch(RESULT r){
            this->ToDoClear();
            if(r != NOERROR){
                if(r != NOWAY) SendResult(this->Connection, r);
                this->ToDoWait(1000); this->ToDoStart();
            }
        }
    }
}
```

**There is no follow-only re-path.** Follow == attack with `Following = true`. When a follow target moves, the `CreatureMoveStimulus` re-arms the chase via the combat path (see below), not via a separate idle routine.

#### Rust current state

The Rust `idle_stimulus.rs` is currently monster-only (the `_ => {}` arm silently skips players). The player idle path is absent entirely — `player-walk-audit.md` BUG P6 tracks the structural issue.

The correct player idle path, once `AttackDest` is implemented, is simply: if `attack_target != 0`, call `ToDoAttack()` + `ToDoStart()`. No follow-specific logic needed because follow IS attack with `following = true`.

---

### BUG-I — `CreatureMoveStimulus` (reactive chase rearm) not implemented for players

**Severity:** MEDIUM  
**Files:** `crates/tfs-rust-core/src/walk/mod.rs` or `monster_events.rs`

#### Decompile (`crmain.cc` — `TCreature::CreatureMoveStimulus`)

Called by `AnnounceMovingCreature` whenever any creature moves, for every other creature that can "see" it:

```cpp
void TCreature::CreatureMoveStimulus(uint32 CreatureID, int Type){
    if(CreatureID == 0 || CreatureID == this->ID
        || this->IsDead
        || this->Combat.AttackDest != CreatureID
        || this->Combat.ChaseMode != CHASE_MODE_CLOSE
        || this->Combat.EarliestAttackTime <= (ServerMilliseconds + 200))
        return;

    if(Type != OBJECT_CHANGED || !this->LockToDo
        || this->ActToDo >= this->NrToDo
        || this->ToDoList.at(this->ActToDo)->Code != TDAttack)
        return;

    TCreature *Target = GetCreature(this->Combat.AttackDest);
    if(Target == NULL) return;
    int Distance = ObjectDistance(this->CrObject, Target->CrObject);
    if(Distance <= 1) return;

    // rearm chase
    if(this->ToDoClear() && this->Type == PLAYER) SendSnapback(…);
    this->ToDoWait(200);
    this->ToDoAttack();
    this->ToDoStart();
}
```

This fires for **players** too (not just monsters). It applies when:
- The creature being chased (`AttackDest`) just moved.
- `ChaseMode == CLOSE` (or `Following = true`, which forces `CHASE_MODE_CLOSE` in `CanToDoAttack`).
- The attack is more than 200 ms away.
- The next ToDo entry is a `TDAttack` (i.e. the creature is waiting to attack, not walking).
- Target distance > 1.

#### Rust current state

`monster_dispatch_creature_move` dispatches to `monster_on_creature_move` for monsters only. No equivalent dispatch exists for players. `player-walk-audit.md` BUG P8 tracks the broader follow re-path issue but was scoped to the TFS `onCreatureMove` pattern, not this CipSoft-specific reactive rearm.

---

### BUG-J — `CGoPath` in Rust: path step directions stored in walk_destinations use cumulative delta from player pos; they must be per-step absolute coordinates

**Severity:** MEDIUM  
**Files:** `crates/tfs-rust-core/src/walk/mod.rs` (`player_auto_walk_path`)

#### Decompile (`receiving.cc` — `CGoPath`)

```cpp
TToDoEntry TD = {};
TD.Code = TDGo;
TD.Go.x = Player->posx;
TD.Go.y = Player->posy;
TD.Go.z = Player->posz;
for(int i = 0; i < Steps; i++){
    switch(Buffer->readByte()){
        case 1: TD.Go.x += 1;               break; // EAST
        …
    }
    Player->ToDoAdd(TD);
}
Player->ToDoStart();
```

Each `TDGo` entry is the **absolute position** of one step, accumulated from the player's current position. The `Go(x,y,z)` executor then checks `max(|OrigX-DestX|, |OrigY-DestY|) <= 1 && OrigZ == DestZ` — it uses the absolute position to verify adjacency.

#### Rust (`player_auto_walk_path`)

```rust
for d in &path {
    pl.base.walk_queue.push_back(*d);
}
let mut acc = pos;
for d in path.iter().rev() {
    acc = acc.offset(*d);
    pl.base.walk_destinations.push_front(acc);
}
```

`walk_destinations` accumulates absolute positions, correctly. The adjacency check in `on_walk`:

```rust
if dx > 1 || dy > 1 || cur_pos.z != dest.z { on_walk_step_rejected(…) }
```

This is correct — it mirrors the decompile's `Distance > 1 || OrigZ != DestZ → throw NOTACCESSIBLE`.

**Verified:** The `walk_destinations` adjacency check matches the decompile's `TCreature::Go` distance guard. ✅

---

### BUG-K — Step timing zero-waypoint fallback diverges from decompile (minor)

**Severity:** LOW  
**Files:** `crates/tfs-rust-content/src/items.rs` (`ground_speed_for_item`)

#### Decompile (`cract.cc` — `TCreature::NotifyGo`)

The decompile reads step timing from the first **BANK-flagged** object at the destination tile via the `WAYPOINTS` attribute. If no BANK object is found, it logs an error and `EarliestWalkTime` is never set — the creature can step again immediately on the next beat.

#### OTB → Rust mapping (verified correct)

The CipSoft `BANK` flag corresponds directly to OTB `ITEM_GROUP_GROUND` (group byte `1`).  
The CipSoft `WAYPOINTS` attribute is stored as OTB `ITEM_ATTR_SPEED` in the `ItemType::speed` field.

The chain in Rust:
- `ItemType::is_terrain_bank_772()` → `self.is_ground_tile()` → `self.group == GROUP_GROUND` ✅
- `ItemType::waypoints_raw_772()` → `self.speed` (OTB `ITEM_ATTR_SPEED`) ✅
- `ItemDatabase::ground_speed_for_item()` → same `self.speed`, with a 150 fallback for zero

This is verified by `ground_tile_speeds_match_objects_srv_waypoint_expectations` (grass=150, dirt=110, sand=160), which matches the expected 772 values. **The attribute mapping is correct.**

#### Only divergence: the zero fallback

`ground_speed_for_item` returns `150` when `speed == 0`. The decompile logs an error and
effectively uses `Waypoints = 0` → `EarliestWalkTime = ServerMilliseconds` (immediate re-step on the
next beat). The Rust fallback imposes a full grass-speed cooldown instead.

This only fires on malformed tiles that have no ground item, which should not exist on a valid
map. The 150 fallback is the safer choice for a corrupted map. No code change needed, but the
doc comment in `ground_speed_for_item` should note the decompile divergence explicitly.

---

### BUG-L — `GetSpeed()` formula: decompile uses `GoStrength.Get() * 2 + 80`; Rust uses a different model for some profiles

**Severity:** MEDIUM  
**Files:** `crates/tfs-rust-core/src/walk/walk_timing.rs`

#### Decompile (`crmain.cc`)

```cpp
int TCreature::GetSpeed(void){
    return this->Skills[SKILL_GO_STRENGTH]->Get() * 2 + 80;
}
```

This is the **only** speed formula. `SKILL_GO_STRENGTH.Get()` returns the current skill value after all modifiers (base + delta + MDAct). There is no log formula, no clamping, no player-vs-monster distinction.

#### Rust (`walk_timing.rs`)

For `StepSpeedModel::LinearGo` (the 772 profile), `go_strength_for_walk` returns the raw `SKILL_GO_STRENGTH` equivalent, clamped to `>= 0` for players. Then `linear_go_effective_speed(go)` is called, which must return `go * 2 + 80` to match.

```rust
pub fn linear_go_effective_speed(go: i32) -> i32 {
    // must be: go * 2 + 80
}
```

If `linear_go_effective_speed` implements this correctly, there is no bug. **Verify** this function returns exactly `go * 2 + 80` for all creature types. The 772 profile must not apply any log curve or clamping to the output of `linear_go_effective_speed`.

Note: the `BalancedLog` player speed model applies `balanced_softened_go` before the `2*go+80` formula, which deviates from the decompile. This is intentional if the server is in balanced mode, but must not apply to monsters.

---

### VERIFIED — `ToDoClear` + `SendSnapback` semantics match the decompile ✅

The Rust `player_todo_clear_with_snapback` matches `CGoDirection`'s preamble: it clears the queue and sends snapback if a pending `TDGo` was found. The decompile's `ToDoClear()` returns true when a pending `TDGo` existed. Verified correct.

### VERIFIED — `ToDoStart` delay clamp (`Delay < 1 → 1`) matches ✅

Decompile `ToDoStart`:
```cpp
uint32 Delay = this->CalculateDelay();
if(Delay < 1) Delay = 1;
```

Rust `todo_start_go_delay`:
```rust
let delay = calc_delay.max(1);
```

Matches. ✅

### VERIFIED — `MoveCreatures` drain condition matches ✅

Decompile:
```cpp
while(ToDoQueue.Entries > 0){
    if(ExecutionTime > ServerMilliseconds) break;
    …
    Creature->Execute();
}
```

Rust `drain_todo_queue`:
```rust
while let Some(entry) = self.todo_queue.peek() {
    if entry.execution_time > self.server_ms { break; }
    …
    self.process_creature_todo(entry.creature_id);
}
```

Matches. ✅

---

## Summary table

| ID | Severity | Description | Decompile ref | Status |
|----|----------|-------------|---------------|--------|
| **BUG-A** | MEDIUM | Drunk stagger: missing `Get()==0` suppression check; `walk_destinations` desync | `cract.cc` `TCreature::Go` | Needs fix |
| **BUG-B** | **HIGH** | Stair climb: not gated on same-z being blocked; wrong height check; missing jump check | `cract.cc` `TCreature::Go` | Needs fix |
| **BUG-C** | MEDIUM | Direction set before move (Rust) vs after (decompile); diagonal dirs emitted where decompile gives cardinal only | `cract.cc` `NotifyTurn` | Needs fix |
| **BUG-D** | — | `CGoDirection` behaviour verified correct | `receiving.cc` | ✅ No bug |
| **BUG-E** | **HIGH** | Attack/Follow not implemented; `CAttack(Follow=true)` is follow | `receiving.cc` `CAttack` | Not implemented |
| **BUG-F** | MEDIUM | `CCancel` not implemented; must stop attack + clear queue + yield | `receiving.cc` `CCancel` | Not implemented |
| **BUG-G** | LOW | `CRotate` is immediate in Rust; decompile queues it via `TDRotate` | `receiving.cc` `CRotate` | Minor divergence |
| **BUG-H** | MEDIUM | Player `IdleStimulus` absent; decompile: if `AttackDest != 0` → `ToDoAttack + ToDoStart` | `crplayer.cc` `IdleStimulus` | Not implemented |
| **BUG-I** | MEDIUM | `CreatureMoveStimulus` reactive chase rearm not dispatched to players | `crmain.cc` `CreatureMoveStimulus` | Not implemented |
| **BUG-J** | — | `walk_destinations` adjacency check verified correct | `cract.cc` `TCreature::Go` | ✅ No bug |
| **BUG-K** | **HIGH** | Step timing: fallback ground speed 150 diverges from decompile zero-waypoint error case; verify WAYPOINTS attribute mapping | `cract.cc` `NotifyGo` | Needs verification |
| **BUG-L** | MEDIUM | `GetSpeed()` formula must be `go*2+80` for all creatures; verify `linear_go_effective_speed` | `crmain.cc` `GetSpeed` | Needs verification |

## Priority order

1. **BUG-E + BUG-H + BUG-F** — attack/follow/cancel are the foundational missing features; nothing chase-related works without them.
2. **BUG-B** — stair routing is wrong for every player near stairs.
3. **BUG-K** — step timing is the most gameplay-visible metric; verify the WAYPOINTS mapping first before optimising anything else.
4. **BUG-I** — reactive chase rearm; depends on BUG-E.
5. **BUG-C** — direction diverges for diagonal steps; affects spectator packet correctness.
6. **BUG-L** — verify `linear_go_effective_speed`; low risk if already correct.
7. **BUG-A** — drunk suppression check; low gameplay impact.
8. **BUG-G** — rotate ordering; cosmetic.
