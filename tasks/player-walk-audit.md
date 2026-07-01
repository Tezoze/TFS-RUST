# Player Movement Audit — Findings & Fixes

**Date:** 2026-07-01 (updated)
**Auditor:** Devin (GLM-5.2 High)
**Scope:** Player-specific walk paths in `walk/mod.rs`, `walk_action.rs`, `walk_tile.rs`, `game_loop.rs`, `creature_think.rs`, `monster_events.rs`
**Reference:** TFS 1.4.2 `src/creature.cpp`, `src/player.cpp`, `src/game.cpp` (1098); **772 = CipSoft decompile** `reference/cipsoft-772/tibia-game-master/src/` (`cract.cc`, `crmain.cc`, `crplayer.cc`, `crcombat.cc`, `receiving.cc`, `sending.cc`, `operate.cc`).

> **Re-audit note (2026-07-01, decompile pass):** The original findings below cited the
> **tvp-772 TFS port** (`reference/tvp-772/gameserver/src/`) for 772 behavior. Per
> `.cursor/rules/TFS-protocol-versioning.mdc`, **772 game mechanics must match the CipSoft
> decompile** (`tibia-game-master/src/`), *not* the tvp/TFS port (which is only authoritative
> for 772 **wire** bytes). This section re-verifies every finding against the decompile and
> records corrections + new findings. `chase_kite_scenario.cc` / `chase_path_debug.cc` are
> in-repo study harnesses, **not** authoritative — do not cite them.

---

## RE-AUDIT (772 decompile) — Corrected Model & Verdicts

### The real 772 walk model (decompile)

772 does **not** have TFS's `listWalkDir` + `eventWalk` timer, nor separate
`followCreature`/`attackedCreature`. It has a **single per-creature ToDo queue** drained by
`TCreature::Execute` (`cract.cc:783`). Players *and* monsters share it.

**Walk request path** (`receiving.cc`):
- `CGoDirection` (single step) / `CGoPath` (autowalk): `if(ToDoClear()) SendSnapback;` → build
  `TDGo` entries → `ToDoStart()`. On thrown `RESULT`: `SendResult(r)` then `ToDoYield()`
  (`receiving.cc:120-199`).
- `CGoStop`: `ToDoStop()` (`receiving.cc:201-211`).
- `CAttack(Follow)` (attack *and* follow are the **same** opcode path):
  `Combat.SetAttackDest(TargetID, Follow)` → `ToDoAttack()` → `ToDoStart()` (`receiving.cc:1133-1155`).

**Step execution** (`TCreature::Go`, `cract.cc:379`):
1. Distance/z guard → `throw NOTACCESSIBLE`.
2. Drunk stagger: `DrunkLevel = Skills[SKILL_DRUNKEN]->TimerValue()`;
   `if(DrunkLevel>0 && Skills[SKILL_DRUNKEN]->Get()==0)` and `rand()%max(7-DrunkLevel,1)==0` →
   pick random cardinal dest, `if(ToDoClear() && PLAYER) SendSnapback;`, `ToDoTalk("Hicks!")`,
   `ToDoStart()` — then still attempts the (random) step this beat.
3. `if(!MovePossible(...))`: player non-diagonal height climb (`GetHeight >= 24`, up then down);
   if still `posz == DestZ` → `throw MOVENOTPOSSIBLE`. **`Go` never sets `Direction`.**
4. `::Move(...)` on success.

**Direction** is set in the global `::Move` via `Creature->NotifyTurn(Con)` (`operate.cc:1407`,
`cract.cc:1541-1557`) — **after** a successful move, from the destination offset with
**X-over-Y priority** (diagonal → E/W, cardinal only). On a blocked move `Go` throws before
`::Move`, so `Direction` is unchanged.

**Step timing** — `NotifyGo` (`cract.cc:1530-1534`):
`Delay = (Waypoints*1000)/GetSpeed(); EarliestWalkTime = now + ceil(Delay/Beat)*Beat`, diagonal
`Waypoints*=3`, `GetSpeed() = GoStrength*2 + 80` (`crmain.cc:477-484`).

**Failed-step handling** — `Execute` catch block (`cract.cc:870-889`):
```cpp
bool SnapbackNecessary = (ToDoClear() || Stop);   // clears WHOLE queue
if(r == EXHAUSTED){ ToDoWait(1000); ToDoStart(); }
else             { ToDoYield(); }                  // ToDoWait(0)+ToDoStart → IdleStimulus
if(Type == PLAYER){
    SendResult(Connection, r);                     // text; for MOVENOTPOSSIBLE also SendSnapback
    if(SnapbackNecessary && r!=MOVENOTPOSSIBLE && r!=NOTINVITED && r!=ENTERPROTECTIONZONE)
        SendSnapback(Connection);
}
```
`SendResult` (`sending.cc:285-357`) sends the **text message first**, then `SendSnapback` for
`MOVENOTPOSSIBLE`/`NOTINVITED`/`ENTERPROTECTIONZONE`. Net wire order on a blocked walk step:
**text → `0xB5` snapback.**

**Resume / re-path** — after `ToDoClear`+`ToDoYield`, the queue drains → `IdleStimulus`:
- `TPlayer::IdleStimulus` (`crplayer.cc:388-405`) handles **only** `Combat.AttackDest`:
  `ToDoAttack(); ToDoStart();` (catch → `ToDoClear` + `ToDoWait(1000)`). There is **no** separate
  follow re-path.
- `TCombat::CanToDoAttack` (`crcombat.cc:442-511`) does the chase: `Following ⇒ ChaseMode=CLOSE`;
  `CLOSE & Distance>1 ⇒ ToDoGo(target, false, 3)` (≤3 steps); `RANGE` keeps distance 4.
- Re-path therefore happens on the **attack beat**, not on target movement. There is **no**
  `onCreatureMove`-driven follow trigger in 772.

`SendSnapback` (`sending.cc:1645-1659`) = `SV_CMD_SNAPBACK` + `Player->Direction` (current facing).

### Current Rust architecture vs the decompile

`creature_uses_todo_execute` (`creature_todo.rs:380`) and `idle_stimulus` /
`request_idle_stimulus` (`idle_stimulus.rs:80,105`) are **monster-only** — the `_ => {}` arm
silently drops players/NPCs. 772 **players** instead run the TFS-style `walk_queue` +
`add_event_walk`, with `clear_todo_772` (`walk/mod.rs:653`) emulating `ToDoClear`+`SendSnapback`
on each new move request. This split is the root cause of P1/P2/P4 and means the decompile's
unified ToDo/`IdleStimulus` path is not exercised for players.

### Per-finding verdicts

| Bug | Verdict | Notes (decompile) |
|-----|---------|-------------------|
| **P1** | ✅ CONFIRMED | `Execute` catch → `ToDoClear()` clears the whole queue (`cract.cc:871`). Rust 772 *player* path only sets `force_update_follow_path` (queue not cleared). Correct C++ refs: `cract.cc:783-889` (Execute), `953-989` (ToDoClear). |
| **P2** | ⚠️ CORRECTED | Resume mechanism is real (`ToDoYield`→`IdleStimulus`), but the described `Player::onIdleStimulus` (separate follow re-path + chase) is **tvp/TFS**. Real 772 `TPlayer::IdleStimulus` handles only `AttackDest`; chase lives in `CanToDoAttack` (`crcombat.cc:442-511`) gated on `ChaseMode`. Follow == attack with `Following=true`. |
| **P3** | ✅ CONFIRMED (rewrite fix) | Attack/Follow/CancelAttackAndFollow still fall through the `_ =>` catch-all (`game_loop.rs:418`). But 772 is **one** path: `SetAttackDest(id, Follow)` + `ToDoAttack` + `ToDoStart`; cancel = `SetAttackDest(0,…)`/`ToDoStop`. No separate `follow_target`/`goToFollowCreature`. Refs: `receiving.cc:1133-1155`, `crcombat.cc:357-440`. |
| **P4** | ⚠️ CORRECTED | Decompile clears the queue and yields to `IdleStimulus` for **all** 772 players on a blocked step, regardless of attack state (`ToDoClear` is unconditional). So the fix is "always clear + idle-stimulus", not "stop only when not attacking". |
| **P5** | ❌ INVALID (Rust is correct for 772) | `Go` never sets `Direction`; it's set post-move by `NotifyTurn` (`operate.cc:1407`). On a blocked move direction is unchanged — **exactly** what Rust does. The "deviation" was measured vs tvp/TFS `internalMoveCreature`. Close it. |
| **P6** | ⚠️ CONFIRMED (root cause) | 772 players *should* share the monster ToDo/`Execute`/`IdleStimulus` path (`creature_uses_todo_execute` is monster-only). This split is why P1/P2/P4 exist. Keep as the structural fix. |
| **P7** | ❌ INVALID (Rust order is correct) | `SendResult` emits **text first**, then `SendSnapback` (`sending.cc:351-356`). Rust already sends text → `0xB5` (`walk/mod.rs:1284-1291`). The "C++ sends cancel first" claim is tvp/TFS. Close it. |
| **P8** | ❌ N/A for 772 (1098-only) | 772 has no `onCreatureMove` follow trigger; chase re-paths on the attack/`IdleStimulus` beat via `CanToDoAttack`. Keep P8 scoped to 1098 only; do **not** add a 772 player-move follow dispatch. |
| **P9** | ❌ N/A for 772 (1098-only) | No per-failed-move `nextAction` lockout in 772; `EarliestWalkTime` updates only in `NotifyGo` on success. Keep scoped to 1098. |

### New findings (decompile pass)

**N1 (MEDIUM): 772 drunk stagger uses TFS formula and skips `ToDoClear`.**
`try_drunk_walk_direction` (`walk/mod.rs:116`) uses TFS 1.4.2 `uniform_random(0,399)` + `r/4 >
drunkenness`. The decompile uses `rand()%max(7-DrunkLevel,1)==0` where `DrunkLevel =
SKILL_DRUNKEN TimerValue` and only when `Skills[SKILL_DRUNKEN]->Get()==0` (`cract.cc:392-413`).
Also, on a stagger the decompile does `ToDoClear()` (aborting the rest of an autowalk) +
`SendSnapback` (player) + `ToDoTalk("Hicks!")` + `ToDoStart()`; the Rust path only rewrites the
direction and says "Hicks!" — it does **not** clear the remaining `walk_queue` or snapback. For a
772 drunk player mid-autowalk this leaves stale steps queued. (Belongs partly to
`MechanicsProfile`.)

**N2 (LOW): 772 `NotifyGo`/`clear_todo_772` reference hygiene.**
`clear_todo_772`, `player_move_request`, `player_auto_walk_path` (`walk/mod.rs:644-755`) cite
`gameserver/src/…` (tvp). The decompile equivalents are `receiving.cc:120-199` (`CGoPath` /
`CGoDirection`: `ToDoClear`→`SendSnapback`→`TDGo`→`ToDoStart`) and `cract.cc:953-1024`
(`ToDoClear`/`ToDoStart`). Re-point the comments (behavior already matches).

**N3 (LOW): step-timing "floor ×2" comment is not in the decompile.**
`linear_go_step_duration_ms` doc + `walk_timing.rs` mention a `2 (floor)` waypoint multiplier.
`NotifyGo` only applies `×3` for diagonal (`cract.cc:1526-1528`); there is no floor multiplier.
The code path only uses 1/3 (`waypoint_step_cost_for_direction`), so this is a comment fix, but
worth deleting to avoid future mis-implementation.

**N4 (INFO): `EarliestWalkTime` source is the tile BANK `WAYPOINTS` attribute.**
`NotifyGo` reads `Waypoints` from the destination BANK object's `WAYPOINTS` attribute
(`cract.cc:1513-1522`), not a hard-coded ground-speed table. Rust's `ground_speed_for_tile_body`
must map to that attribute for exact step-timing parity (verify `items_db.ground_speed_for_item`
returns the CipSoft `WAYPOINTS` value on 772). No code change if the item DB already carries it.

**Corrected fix order (772):** implement the shared ToDo/`IdleStimulus` path for players (**P6**)
which subsumes **P1/P2/P4**; then **P3** (unified `SetAttackDest`+`ToDoAttack` handler); **P8/P9**
remain 1098-only; **P5/P7** are closed as invalid; **N1** with the mechanics profile.

---

## ARCHITECTURE DECISION — Unify BOTH eras on the CipSoft decompile walk engine

**Decision (2026-07-01):** Adopt the CipSoft 772 decompile walk model
(`TCreature::Execute` ToDo queue → `Go`/`Attack` → `IdleStimulus` → `Combat.CanToDoAttack`) as
the **single walk/chase engine for both 772 and 1098**. Retire the TFS-style parallel paths:
`listWalkDir`/`eventWalk` deadline scheduling, the 772-player `walk_queue` split, and the
`goToFollowCreature`/`onThink`-poll/`onCreatureMove` follow machinery.

**Rationale:** TFS's own walk logic was a reverse-engineering *attempt* to reproduce CipSoft
behavior, so the decompile is the more faithful spec, not a divergent one. The OTClient accepts
**both** timing/authority models — tvp-772 already drives a 772 client with TFS-derived (non-CipSoft)
logic — so the 10.98 client's local prediction tolerates the decompile's server-authoritative
ToDo model. The era differences are **numeric/config, not structural control flow**, so one engine
+ a `MechanicsProfile` covers both. Downgrades the "1098 prediction desync" risk I raised earlier
from *blocker* to *validate-in-QA*.

### What unifies vs what stays era-specific

| Concern | Unify (one code path) | Keep era-specific (where) |
|---------|-----------------------|---------------------------|
| Scheduling model | `Execute` ToDo queue + `IdleStimulus` drain for players **and** monsters | — |
| Failed-step handling | `ToDoClear` + `ToDoYield` → `IdleStimulus` | — |
| Follow / chase | `SetAttackDest(id, Follow)` + `CanToDoAttack` (`ChaseMode`) | — |
| Step-duration **formula** | shared `get_step_duration` seam | 772 linear `Waypoints*1000/GetSpeed` ceil→Beat vs 1098 log `A·ln+C` → `MechanicsProfile` |
| Effective speed | shared | `2·go+80` (772) vs clamped log speed (1098) → `MechanicsProfile` |
| Scheduler **clock** | shared `ToDoQueue` API | discrete Beat (`server_ms`) vs continuous (`Instant`) → clock adapter, quantize-to-Beat only when profile sets it |
| Wire bytes | — | snapback `0x6B`/`0xB5`, move `0x6D` stay in the `ProtocolCodec` |

### Migration plan → `tasks/walk-engine-unification.md`

The phased, checklist-style migration plan lives in **`tasks/walk-engine-unification.md`**
(Phase 0 clock seam → **Phase 1 = 772 player parity, the immediate focus** → Phase 2 = 1098,
deferred). It maps each audit bug/finding to concrete tasks with file locations, decompile refs,
and acceptance criteria. Summary of how the findings land there:
- **P6 (+P1/P2/P4)** and **P3** are closed in Phase 1 when 772 players move onto the ToDo path.
- **P8/P9** dissolve in Phase 2 into `CanToDoAttack` / `EarliestWalkTime` (no longer special cases).
- **P5/P7** stay closed; **N1** is Phase 1.5; **N2/N3/N4** are Phase 1.6 cleanup.

### Risk to validate in QA (not a blocker)

1098 step-timing *feel* / client prediction under the ToDo model — verify against a live 10.98
client. Mitigation: the 1098 **log** step-duration stays in `MechanicsProfile`, so per-step timing
is preserved even though the control flow unifies; only the *scheduling mechanism* changes.

---

## Do Players and Monsters Use the Same Walk Code?

**Yes — they share `on_walk`** (`walk/mod.rs:1195`), with branches:

| Aspect | Player | Monster |
|--------|--------|---------|
| Step pop | `walk_queue.pop_back()` always | `walk_queue.pop_back()` if non-empty, else `monster_next_walk_step()` |
| Drunk walk | Yes (player-only check) | No |
| Pre-step kick | `Proceed` (no-op) | `monster_push_before_step()` |
| Error: 772 | `force_update_follow_path` if following (BUG P1: queue NOT cleared) | Clear queue + `request_idle_stimulus` |
| Error: 1098 | `force_update_follow_path` if following | Same as player |
| Success: direction | `set_direction_from_step` + chain turn | Same |
| Success: move packet | `emit_move_packet` + `broadcast_spectator_move` | Same |
| Reschedule | `add_event_walk` | 772: todo execute; 1098: `schedule_walk_followup_deadline` or stop |
| onCreatureMove fan-out | **Missing** (BUG P8) | `monster_dispatch_creature_move` → `monster_on_creature_move` |

The shared code means bugs in `on_walk` affect both, but the error handling branches diverge significantly for 772. Additionally, the `onCreatureMove` fan-out after a successful move only dispatches to monsters — players never receive the event.

---

## The "Turn Packet on Obstacle" Question

**There is NO `0x6B` creature turn broadcast on failed moves.** The packet sent is `0xB5` (cancel walk), which contains a direction byte but is a distinct opcode:

| Packet | Opcode | Recipient | Purpose |
|--------|--------|-----------|---------|
| `0xB5` Cancel Walk | `0xB5` | Moving player only | Stop walk animation, face direction |
| `0x6B` Creature Turn | `0x6B` | All spectators | Broadcast facing direction change |

**All `encode_cancel_walk` calls send only to the player's own connection** — verified at all 5 call sites in `walk/mod.rs` (lines 669, 697, 736, 1290, 1457). None broadcast to spectators.

The `0x6B` turn broadcast only happens on **successful** moves:
- `set_direction_from_step` in `internal_move_creature_step` (line 1614) — sets direction silently (no broadcast)
- `internal_creature_turn_with_broadcast` (line 1671) — broadcasts `0x6B` for post-queryDestination chain turns (floor changes only)
- `creature_turn_with_broadcast` — called from monster idle rotate and sim harness only

**Conclusion:** The "turn packet on obstacle" is the `0xB5` cancel walk, which is correct behavior — it tells the client to stop walking and face the current direction. It does NOT go to other creatures/players. The `0xB5` direction byte echoes the player's **current** facing (not the attempted move direction), which is also correct — the player's direction was not changed by the failed move.

---

## Findings

> **Superseded by the RE-AUDIT section above for 772.** The C++ snippets below are from the
> **tvp-772 TFS port** and are kept for context only. For 772 parity use the decompile refs and
> verdicts in "RE-AUDIT (772 decompile)". 1098 details below remain accurate.

### BUG P1 (HIGH): 772 player `walk_queue` NOT cleared on failed move

**File:** `crates/tfs-rust-core/src/walk/mod.rs` lines 1296–1318

**C++ 772 behavior:** `Game::moveCreature` (game.cpp:802–817) on failed move:
```cpp
player->sendCancelWalk();
if (player->clearToDo() && !player->attackedCreature) {
    creature->stopToDo();    // stops execution, sends another sendCancelWalk()
}
if (player->attackedCreature || player->followCreature) {
    player->addWaitToDo(0);  // immediate resume
    player->startToDo();
}
player->sendCancelMessage(ret);
```

`clearToDo()` (creature.cpp:1351–1366) clears **all** ToDo entries (walk + action):
```cpp
bool Creature::clearToDo() {
    bool cancelWalk = false;
    for (const ToDoEntry& entry : toDoEntries) {
        if (entry.type == TODO_WALK) cancelWalk = true;
    }
    toDoEntries.clear();
    isExecuting = false;
    currentToDo = 0;
    totalToDo = 0;
    stopExecuting = false;
    return cancelWalk;
}
```

**Rust behavior:** The 772 player error path (line 1314–1318):
```rust
} else if let Some(k) = self.creatures.get_mut(cid) {
    if k.base().follow_target.is_some() {
        k.base_mut().force_update_follow_path = true;
    }
}
```

Only sets `force_update_follow_path` if following. The `walk_queue` is **NOT cleared**. The walk timer keeps firing with stale steps.

**Impact:**
- 772 player auto-walks into a wall → remaining steps keep firing → repeated `0xB5` + text messages
- Player stutters against the obstacle instead of stopping cleanly
- Walk timer wastes cycles popping stale directions

**Fix:** Clear `walk_queue` and stop the walk timer for 772 players on failed move:
```rust
if self.beat_driven_loop && self.creatures.get(cid)
    .is_some_and(|k| matches!(k, CreatureKind::Player(_)))
{
    if let Some(k) = self.creatures.get_mut(cid) {
        let base = k.base_mut();
        base.walk_queue.clear();
        if base.follow_target.is_some() {
            base.force_update_follow_path = true;
        }
    }
    self.stop_event_walk(cid);
    // Resume follow/attack if needed (see BUG P2)
}
```

---

### BUG P2 (HIGH): 772 player follow/attack resume missing on failed move

**File:** `crates/tfs-rust-core/src/walk/mod.rs` lines 1296–1318

**C++ 772 behavior:** When a following/attacking player's move fails (game.cpp:812–815):
```cpp
if (player->attackedCreature || player->followCreature) {
    player->addWaitToDo(0);   // TODO_WAIT with time=0
    player->startToDo();      // resume execution
}
```

This triggers `executeToDoEntries` → TODO_WAIT(0) executes immediately → loop ends → `onIdleStimulus()` → `Player::onIdleStimulus` (player.cpp:1265–1306):
```cpp
void Player::onIdleStimulus() {
    if (followCreature) {
        // re-path to follow target
        if (!Position::areInRange<1, 1>(myPos, targetPos)) {
            if (getPathTo(targetPos, dirList, 0, 1, true, true, 10)) {
                addWaitToDo(100);
                addWalkToDo(dirList);
            }
        }
    }
    if (attackedCreature) {
        // chase mode: walk toward target (max 3 steps)
        if (chaseMode && !Position::areInRange<1, 1>(myPos, targetPos)) {
            if (getPathTo(targetPos, dirList, 0, 1, true, true, 10)) {
                addWalkToDo(dirList, 3);
            }
        }
        addAttackToDo();
    }
    startToDo();
}
```

The player **immediately re-paths and resumes following/attacking** — all within the same ToDo execution cycle.

**Rust behavior:** No player idle stimulus exists. `force_update_follow_path = true` is set, but the repath only happens on the next `onThink` (1000ms interval) via `go_to_follow_creature` in `creature_think.rs:195–197`.

**Impact:**
- 772 player following a creature that moves behind a wall → player hits wall → waits up to 1000ms before re-pathing
- C++ 772 re-paths within a few scheduler ticks (~50ms)
- Player chase feels sluggish and unresponsive compared to C++ 772

**Fix:** Implement `player_idle_stimulus` for 772 that mirrors `Player::onIdleStimulus`:
```rust
fn player_idle_stimulus_772(&mut self, cid: CreatureId) {
    let (follow_id, attack_id, pos) = match self.creatures.get(cid) {
        Some(CreatureKind::Player(p)) => (p.base.follow_target, p.base.attack_target, p.base.position),
        _ => return,
    };
    if let Some(follow_id) = follow_id {
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return,
        };
        if !are_in_range_1_1(pos, target_pos) {
            if let Some(path) = self.get_creature_path_to(cid, target_pos, 0, 1) {
                // queue new walk steps
                if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                    p.base.walk_queue.clear();
                    for d in path { p.base.walk_queue.push_back(d); }
                }
                self.add_event_walk(cid, true, Instant::now());
            }
        }
    }
    // Attack queueing similar to follow...
}
```

Call this from the 772 player failed-move path when `follow_target.is_some() || attack_target.is_some()`.

---

### BUG P3 (HIGH): Player attack/follow packets not implemented

**File:** `crates/tfs-rust-core/src/game_loop.rs` lines 418–422

**C++ behavior:** `GamePacket::Attack`, `GamePacket::Follow`, `GamePacket::CancelAttackAndFollow` are handled in the game loop dispatch.

**1098 flow** (`src/game.cpp`):
- `playerSetAttackedCreature(id, creatureId)` → `player->setAttackedCreature(creature)` → if chaseMode, `setFollowCreature(creature)` → dispatches `updateCreatureWalk` → `goToFollowCreature()` → path → `startAutoWalk()`
- `playerFollowCreature(id, creatureId)` → `player->setAttackedCreature(nullptr)` → dispatch `updateCreatureWalk` → `player->setFollowCreature(creature)` → `goToFollowCreature()` → path → `startAutoWalk()`
- `playerCancelAttackAndFollow(id)` → `playerSetAttackedCreature(id, 0)` + `playerFollowCreature(id, 0)` + `player->stopWalk()` (sets `cancelNextWalk = true`)

**772 flow** (`gameserver/src/game.cpp`):
- `playerSetAttackedCreature(id, creatureId)` → `player->setFollowCreature(nullptr)` + `player->setAttackedCreature(creature)` + `player->addYieldToDo()` (= `addWaitToDo(50)` + `startToDo()`) → ToDo execution → `onIdleStimulus` → path to target
- `playerFollowCreature(id, creatureId)` → `player->setAttackedCreature(nullptr)` + `player->setFollowCreature(creature)` (which calls `addWaitToDo(100)` + `startToDo()`) → ToDo execution → `onIdleStimulus` → path to follow target
- `playerCancelAttackAndFollow(id)` → `playerSetAttackedCreature(id, 0)` + `playerFollowCreature(id, 0)` + `player->stopToDo()` (stops ToDo execution, sends `sendCancelWalk()`)

**Key 772 difference:** `Player::setFollowCreature` (player.cpp:2904–2918) calls `addWaitToDo(100)` + `startToDo()` on success, which triggers the ToDo execution loop → `onIdleStimulus` → re-path. The 772 `Creature::setFollowCreature` (creature.cpp:594–614) does NOT call `goToFollowCreature` — it just sets the field and calls `onFollowCreature`.

**Rust behavior:** All three are caught by the catch-all:
```rust
_ => trace!(
    conn_id = conn_id.0,
    ?packet,
    "game packet — simulation Phase 9+"
),
```

No player follow/attack target setting exists. `player_set_follow_creature`, `player_set_attack_creature`, `player_cancel_attack_and_follow` — none of these functions exist.

**Impact:**
- Players cannot follow or attack creatures
- No chase mode for players
- `follow_target` and `attack_target` are never set for players (except in tests)
- BUG P2's resume logic has nothing to resume
- BUG P8's `onCreatureMove` trigger has no follow target to check

**Fix:** Implement the three packet handlers with era-specific behavior:

1. `GamePacket::Attack { creature_id }`:
   - 1098: set `attack_target`, if chaseMode set `follow_target` → `go_to_follow_creature` → `add_event_walk`
   - 772: clear `follow_target`, set `attack_target` → schedule idle stimulus (ToDo resume)

2. `GamePacket::Follow { creature_id }`:
   - 1098: clear `attack_target`, set `follow_target` → `go_to_follow_creature` → `add_event_walk`
   - 772: clear `attack_target`, set `follow_target` → schedule idle stimulus (ToDo resume)

3. `GamePacket::CancelAttackAndFollow`:
   - 1098: clear both targets, set `cancel_next_walk = true`
   - 772: clear both targets, `stop_event_walk`, send `0xB5`

Reference: TFS 1.4.2 `Game::playerSetAttackedCreature` (game.cpp:3237), `Game::playerFollowCreature` (game.cpp:3269), `Game::playerCancelAttackAndFollow` (game.cpp:3225). 772: `Game::playerSetAttackedCreature` (game.cpp:3028), `Game::playerFollowCreature` (game.cpp:3066), `Game::playerCancelAttackAndFollow` (game.cpp:3015).

---

### BUG P4 (MEDIUM): 772 player `stopToDo` on failed move while not attacking/following

**File:** `crates/tfs-rust-core/src/walk/mod.rs` lines 1296–1318

**C++ 772 behavior:** When a non-attacking, non-following player's move fails (game.cpp:807–809):
```cpp
if (player->clearToDo() && !player->attackedCreature) {
    creature->stopToDo();
}
```

`stopToDo()` (creature.cpp:1342–1349) sets `stopExecuting = true` if currently executing, or sends `sendCancelWalk()` if not. This stops the ToDo execution loop.

**Rust behavior:** For 772 players not following, nothing happens — no `walk_queue.clear()`, no `stop_event_walk()`.

**Impact:** A 772 player auto-walking into a wall continues to poll the walk timer for each remaining step, sending repeated cancel messages.

**Fix:** Combined with BUG P1 fix — clear `walk_queue` and `stop_event_walk` for all 772 players on failed move, regardless of follow/attack state.

---

### BUG P5 (MEDIUM): Direction not set before height-based floor change on failed move

**File:** `crates/tfs-rust-core/src/walk/walk_tile.rs` lines 115–191

**C++ behavior (both 772 and 1098):** `Game::internalMoveCreature` (game.cpp:815, 829) sets `player->setDirection(direction)` **before** the `queryAdd` check:
```cpp
// try to go up
if (!tmpTile->hasFlag(TILESTATE_FLOORCHANGE)) {
    player->setDirection(direction);  // ← SET BEFORE MOVE
    destPos.z--;
}
// ...
// try to go down
player->setDirection(direction);  // ← SET BEFORE MOVE
destPos.z++;
```

If the subsequent `queryAdd` fails, the direction has already been changed. `sendCancelWalk()` then sends the **new** direction, causing the client to turn even though the player didn't move.

**Rust behavior:** `resolve_player_move_destination` only computes the destination position and flags — it does NOT set direction. Direction is set only on success in `internal_move_creature_step` (line 1614–1620).

**Impact:** This is actually a **Rust improvement** — the player doesn't turn when walking into an obstacle on a stairs tile. However, it's a behavioral difference from C++ that could cause desync with clients that expect the direction change.

**Fix:** This is an accepted deviation. Document it. If strict parity is needed, set direction in `resolve_player_move_destination` for height-change cases (not recommended — the Rust behavior is more correct).

---

### BUG P6 (LOW): 772 player walk uses `walk_queue` + `add_event_walk` instead of ToDo system

**File:** `crates/tfs-rust-core/src/walk/mod.rs` lines 678–714

**C++ 772 behavior:** Player walk goes through the ToDo system:
1. `playerMove` → `addWalkToDo(dir)` → `startToDo()`
2. `executeToDoEntries` → `Game::moveCreature(creature, dir, flags)`
3. Each step is a ToDo entry with a bound function

**Rust behavior:** 772 player walk uses `walk_queue` + `add_event_walk` (same as 1098):
1. `player_move_request` → `walk_queue.push_back(dir)` → `add_event_walk`
2. Walk timer fires → `on_walk` → `internal_move_creature_step`

The Rust code unifies walk scheduling but misses 772-specific ToDo behaviors:
- `clearToDo()` clearing all entries on failure (BUG P1)
- `stopToDo()` stopping execution (BUG P4)
- `addWaitToDo(0)` + `startToDo()` resume (BUG P2)
- `onIdleStimulus` re-pathing (BUG P2)

**Impact:** The unified approach is cleaner but the 772-specific error handling and resume logic must be replicated in the `on_walk` error path.

**Fix:** No structural change needed — add the 772-specific error handling to `on_walk` as described in BUGs P1, P2, P4.

---

### BUG P7 (LOW): `0xB5` cancel walk order differs from C++ 772

**File:** `crates/tfs-rust-core/src/walk/mod.rs` lines 1284–1291

**C++ 772 behavior:** `Game::moveCreature` on failed move (game.cpp:802–817):
```cpp
player->sendCancelWalk();         // 1st: 0xB5
// clearToDo, stopToDo/resume
player->sendCancelMessage(ret);   // 2nd: text message
```

**Rust behavior:** `on_walk` error path (lines 1284–1291):
```rust
self.enqueue_outgoing(conn, send_text_message_simple(MESSAGE_STATUS_SMALL, msg));  // 1st: text
self.enqueue_outgoing(conn, self.codec.encode_cancel_walk(d as u8));               // 2nd: 0xB5
```

The order is reversed — Rust sends text message first, then cancel walk. C++ sends cancel walk first, then text message.

**Impact:** Minimal — the client processes both packets in the same tick. But for strict parity, the order should match C++.

**Fix:** Swap the order:
```rust
self.enqueue_outgoing(conn, self.codec.encode_cancel_walk(d as u8).into_bytes());
self.enqueue_outgoing(conn, send_text_message_simple(MESSAGE_STATUS_SMALL, msg).into_bytes());
```

---

### BUG P8 (HIGH): 1098 `Player::onCreatureMove` follow re-path trigger missing

**File:** `crates/tfs-rust-core/src/monster_events.rs` lines 572–583; `crates/tfs-rust-core/src/walk/mod.rs` line 1710

**C++ 1098 behavior:** `Player::onCreatureMove` (player.cpp:1346–1354) is called for every creature move in the player's viewport:
```cpp
void Player::onCreatureMove(Creature* creature, const Tile* newTile, const Position& newPos,
                            const Tile* oldTile, const Position& oldPos, bool teleport)
{
    Creature::onCreatureMove(creature, newTile, newPos, oldTile, oldPos, teleport);

    if (hasFollowPath && (creature == followCreature || (creature == this && followCreature))) {
        isUpdatingPath = false;
        g_dispatcher.addTask(createTask(std::bind(&Game::updateCreatureWalk, &g_game, getID())));
    }
    // ... trade close checks ...
}
```

When the follow target moves (or the player themselves moves while following), `isUpdatingPath` is cleared and `updateCreatureWalk` is dispatched → `goToFollowCreature()` → re-path → `startAutoWalk()`. This is the **real-time follow mechanism** — the player re-paths immediately when the target moves, not on a polling interval.

**Rust behavior:** `move_creature_on_map` (walk/mod.rs:1710) only calls `monster_dispatch_creature_move` — there is no player equivalent. The `onCreatureMove` fan-out is monsters-only:
```rust
pub(crate) fn move_creature_on_map(&mut self, cid: CreatureId, from: Position, to: Position) {
    // ...
    self.monster_dispatch_creature_move(cid, from, to);
    // ← no player_dispatch_creature_move
}
```

Player follow re-pathing relies solely on `creature_think.rs` polling `FOLLOW_PATH_UPDATE_INTERVAL_MS` (200ms) via `onThink`. This is a 200ms delay vs C++'s immediate re-path.

**Impact:**
- 1098 player following a creature that kites → up to 200ms delay before re-pathing
- C++ 1098 re-paths within the same dispatcher tick (~1ms)
- Follow feels laggy compared to C++ 1098, especially against fast-moving targets
- Combined with BUG P3 (no follow at all), this is currently unobservable, but will matter once P3 is fixed

**Fix:** Add `player_dispatch_creature_move` alongside `monster_dispatch_creature_move`:
```rust
pub(crate) fn move_creature_on_map(&mut self, cid: CreatureId, from: Position, to: Position) {
    // ...
    self.monster_dispatch_creature_move(cid, from, to);
    self.player_dispatch_creature_move(cid, from, to);
}

fn player_dispatch_creature_move(&mut self, moved: CreatureId, old_pos: Position, new_pos: Position) {
    // Collect players witnessing the move (same spatial logic as monsters)
    let players = self.players_witnessing_move(old_pos, new_pos);
    for player_id in players {
        self.player_on_creature_move(player_id, moved, old_pos, new_pos);
    }
}

fn player_on_creature_move(&mut self, player_id: CreatureId, moved: CreatureId, _old: Position, _new: Position) {
    let Some(CreatureKind::Player(p)) = self.creatures.get(player_id) else { return; };
    let has_follow = p.base.follow_target.is_some();
    let follow_id = p.base.follow_target;
    let has_follow_path = p.base.has_follow_path;
    drop(p); // release borrow

    if !has_follow || !has_follow_path { return; }

    // C++: creature == followCreature || (creature == this && followCreature)
    if moved == follow_id.unwrap_or(player_id) || moved == player_id {
        // Clear isUpdatingPath and re-path immediately
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(player_id) {
            p.base.is_updating_path = false;
        }
        self.go_to_follow_creature(player_id, Some("on_creature_move"));
    }
}
```

Note: `is_updating_path` field exists on `CreatureBase` but is only used for 1098 monsters. The player path needs to gate on it too.

---

### BUG P9 (LOW): 1098 `nextAction` lockout not set on failed move

**File:** `crates/tfs-rust-core/src/walk/mod.rs` lines 1627–1641

**C++ 1098 behavior:** `Player::onWalk(Direction& dir)` (player.cpp:1339–1344) is called from `Creature::getNextStep` **before** `internalMoveCreature`:
```cpp
void Player::onWalk(Direction& dir) {
    Creature::onWalk(dir);
    setNextActionTask(nullptr);
    setNextAction(OTSYS_TIME() + getStepDuration(dir));  // ← set BEFORE move attempt
}
```

Even if the move fails, `setNextAction` has already been called — the player's action lockout is active.

**Rust behavior:** `next_action_until` is set in `internal_move_creature_step` (line 1639) **only on success** — after the move completes. If the move fails, `next_action_until` is not updated.

**Impact:** A 1098 player who walks into an obstacle can immediately send an action (use item, attack) without waiting for the step duration. In C++, they must wait `getStepDuration(dir)` ms even on failed moves.

**Fix:** Set `next_action_until` before the `internal_move_creature_step` call in `on_walk`, matching C++ `Player::onWalk(dir)` timing:
```rust
// In on_walk, after popping dir but before internal_move_creature_step:
if !self.beat_driven_loop {
    if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
        let gs = self.map.get_tile(p.base.position)
            .map(|t| ground_speed_for_tile_body(t.body(), self.items_db.as_ref()))
            .unwrap_or(150);
        let dur = get_step_duration_ms_with_direction(
            /* k */, /* base */, dir, gs, &self.mechanics,
        );
        p.next_action_until = Some(self.now_ms().saturating_add(dur.max(1) as u64));
    }
}
```

Then remove the duplicate set in `internal_move_creature_step` (line 1637–1641).

---

## Priority & Fix Order

> **Note:** This table reflects the original tvp-based pass. See the RE-AUDIT verdicts table for
> the decompile-corrected status (P5/P7 closed as invalid; P8/P9 are 1098-only; P6 subsumes
> P1/P2/P4 via the shared ToDo/`IdleStimulus` path; plus new findings N1–N4).

| Bug | Severity | Impact | Fix Effort | Dependencies |
|-----|----------|--------|------------|--------------|
| **P3** | HIGH | Players can't attack/follow at all | Large | None — foundational |
| **P8** | HIGH | 1098 follow re-path delayed up to 200ms | Medium | P3 (needs follow to exist) |
| **P1** | HIGH | 772 players stutter on obstacles | Small | None |
| **P2** | HIGH | 772 follow/attack doesn't resume | Medium | P3 (needs follow/attack to exist) |
| **P4** | MEDIUM | 772 players poll on failed auto-walk | Small | P1 (same fix block) |
| **P9** | LOW | 1098 action lockout missing on failed move | Small | None |
| **P7** | LOW | Packet order difference | Trivial | None |
| **P5** | LOW | Accepted deviation — no fix | N/A | N/A |
| **P6** | LOW | Structural — no fix needed | N/A | P1, P2, P4 handle the gaps |

**Recommended fix order (original, tvp-based):** ~~P3 → P1+P4 → P8 → P2 → P9 → P7~~ — **superseded**.

> **Superseded by the ARCHITECTURE DECISION** (unify both eras on the CipSoft decompile walk
> engine). Under that plan the per-bug track below collapses into the migration steps: P6 (+P1/P2/P4)
> and P3 land when players move onto the ToDo path; **P8/P9 dissolve** into `CanToDoAttack` /
> `EarliestWalkTime` rather than being fixed as 1098 special cases; **P5/P7** stay closed. The
> per-bug view below is kept only as a fallback if the unify effort is deferred.

### Fallback fix order (only if the unify effort is deferred)

**772 track (decompile parity):**
1. **P3** — foundational. Add the *unified* handler: `SetAttackDest(id, Follow)` → `ToDoAttack` → `ToDoStart` (attack **and** follow are the same path; cancel = `SetAttackDest(0)`). Nothing to resume/chase exists until this lands.
2. **P6** — route 772 players through the shared ToDo / `Execute` / `IdleStimulus` path (`creature_uses_todo_execute` + `idle_stimulus` are currently monster-only). This **subsumes P1 + P4** (blocked step → `ToDoClear` clears the whole queue) **and P2** (`ToDoYield` → `TPlayer::IdleStimulus` → `CanToDoAttack` chase re-path).
   - *If the full P6 refactor is deferred:* land a targeted **P1+P4** first (always clear `walk_queue` + emit a player idle-stimulus on a blocked 772 step, regardless of attack state), then **P2** (chase resume via the `CanToDoAttack` equivalent, `ChaseMode`-gated).
3. **N1** — 772 drunk stagger formula + `ToDoClear`-on-stagger (pair with `MechanicsProfile`).

**1098 track (independent of 772, depends on P3's follow target):**
4. **P8** — real-time `onCreatureMove` follow re-path (1098 only — do **not** add for 772).
5. **P9** — `nextAction` lockout on failed move (1098 only).

**Reference/comment cleanup (no behavior change):** N2, N3, N4.

**Closed — no fix (Rust already matches the decompile):** **P5**, **P7**.

**Rationale:** For 772, P6 is the real fix — the player/monster ToDo split is the root cause of P1/P2/P4, so implementing the shared path resolves all three at once (with a targeted P1+P4→P2 fallback if the refactor is too large for one pass). P3 must come first because follow/attack targets don't exist otherwise. P8/P9 are strictly 1098 behaviors and must not be ported to the 772 path. P5 and P7 were artifacts of comparing against the tvp/TFS port and require no change.

---

## Verification

After each fix, run:
```bash
rtk cargo check
rtk cargo clippy
rtk cargo test -p tfs-rust-core
```

Specifically watch for:
- `step_speed_tests` in `walk/mod.rs` (lines 1802+)
- Player walk tests in `walk_action.rs`
- 772 ToDo tests in `creature_todo.rs`

For behavioral verification, test with:
- 772 player auto-walking into a wall (BUG P1, P4)
- 772 player following a creature that moves behind a wall (BUG P2)
- 1098 player following a creature that kites (BUG P8 — should re-path immediately, not 200ms later)
- Player attacking a creature (BUG P3)
- Player following a creature (BUG P3)
- 772 player on stairs hitting an obstacle (BUG P5)
- 1098 player walking into wall then immediately using an item (BUG P9 — should be locked out for step duration)

---

## Cross-Reference: Monster Walk Audit

See `tasks/walk-audit-findings.md` for the monster-specific walk audit covering:
- BUG 1 (HIGH): `on_walk` bypasses `Monster::getNextStep` idle/dead check
- BUG 2 (HIGH): 1098 monster walk timer re-arms when queue empty
- BUG 3 (MEDIUM): 772 monster `forceUpdateFollowPath` not set on blocked move
- BUG 4 (MEDIUM): `last_step_cost` computed from overall old→new, not last chain segment
- BUG 5 (LOW): `linear_go_speed_from_profile` applies `player_speed_model` to monsters
- BUG 6 (LOW): `schedule_walk_followup_deadline` recomputes delay instead of 1ms poll

The monster audit and this player audit share the same `on_walk` entry point. Bugs in the shared code (e.g., BUG 1 idle check bypass) affect both players and monsters, while the branch-specific bugs (P1–P9 for players, 1–6 for monsters) are isolated to their respective creature types.
