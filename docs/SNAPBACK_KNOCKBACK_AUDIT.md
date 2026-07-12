# 772 Snapback / Knock-Back Audit — `SendSnapback` (0xB5)

Audit of the Rust player-movement "knock back / bounce back" function against the
772 decompile. The bounce-back is `SendSnapback` — opcode `0xB5` + player direction
byte — which tells the client to snap the player back to the server-known position
and facing.

## Wire encoding parity — ✓

| Source | Code |
|--------|------|
| C++ `sending.cc:1645-1659` | `SendByte(SV_CMD_SNAPBACK=0xB5); SendByte(Player->Direction)` |
| TVP `protocolgame.cpp:1501-1507` | `addByte(0xB5); addByte(player->getDirection())` |
| Rust `codec/v772.rs:376-382` | `m.write_u8(0xB5); m.write_u8(direction)` |

Wire bytes match exactly across all three sources.

## C++ 772 snapback call sites (14 total)

| # | File:Line | Context | Snapback condition |
|---|-----------|---------|-------------------|
| 1 | `receiving.cc:135` | `CGoPath` preamble | `if(ToDoClear())` |
| 2 | `receiving.cc:185` | `CGoDirection` preamble | `if(ToDoClear())` |
| 3 | `receiving.cc:1340` | `CCancelAttack` | `if(ToDoClear())` |
| 4 | `cract.cc:406` | `Go` drunk stagger | `if(ToDoClear() && PLAYER)` |
| 5 | `cract.cc:635` | `Move` executor out-of-range | `if(ToDoClear() && PLAYER)` |
| 6 | `cract.cc:743` | `Use` executor out-of-range | `if(ToDoClear() && PLAYER)` |
| 7 | `cract.cc:800` | `Execute` Delay>0 + Stop flag | unconditional (player) |
| 8 | `cract.cc:885` | `Execute` catch, non-exempt result | `SnapbackNecessary && r ∉ {MOVENOTPOSSIBLE,NOTINVITED,ENTERPROTECTIONZONE}` |
| 9 | `cract.cc:894` | `Execute` post-success Stop | unconditional (player) |
| 10 | `cract.cc:994` | `ToDoAdd` locked preamble | `if(ToDoClear() && PLAYER)` |
| 11 | `cract.cc:1006` | `ToDoStop` not-locked | unconditional (player) |
| 12 | `cract.cc:1102` | `ToDoGo` NOWAY (pathfinding fail) | unconditional (player) |
| 13 | `crmain.cc:953` | `CreatureMoveStimulus` (combat rearm) | `if(ToDoClear() && PLAYER)` |
| 14 | `sending.cc:354` | `SendResult` for 3-result set | unconditional (inside `SendResult`) |

### The 3-result snapback interaction (sites 8 + 14)

C++ splits the snapback decision across TWO functions:

**`SendResult`** (`sending.cc:351-356`) — sends message + snapback for the 3 results:
```cpp
SendMessage(Connection, TALK_FAILURE_MESSAGE, "%s", Message);
if(r == ENTERPROTECTIONZONE || r == NOTINVITED || r == MOVENOTPOSSIBLE){
    SendSnapback(Connection);   // ← snapback for these 3
}
```

**`Execute` catch** (`cract.cc:871-886`) — `SnapbackNecessary = ToDoClear() || Stop`:
```cpp
SendResult(this->Connection, r);   // sends message + snapback(if 3-result)
if(SnapbackNecessary && r != MOVENOTPOSSIBLE && r != NOTINVITED && r != ENTERPROTECTIONZONE){
    SendSnapback(this->Connection);  // snapback for OTHER results, only if remaining Gos
}
```

**Net per result for a player in the Execute catch:**
- `MOVENOTPOSSIBLE` / `NOTINVITED` / `ENTERPROTECTIONZONE`: 1 message + **1 snapback** (from `SendResult`). Execute catch skips to avoid double.
- Other results (e.g. `NOTACCESSIBLE`): 1 message + **1 snapback if `SnapbackNecessary`** (remaining Go entries), else 0.

`SnapbackNecessary` is true iff `ToDoClear()` found a `Go` at index `i >= ActToDo` — i.e. there are **remaining unexecuted Go steps** after the one that threw. A single-step walk that throws has `SnapbackNecessary = false`.

## Rust coverage

| C++ # | Rust equivalent | File | Status |
|-------|----------------|------|--------|
| 1 | `player_auto_walk_path` → `player_todo_clear_with_snapback` | `walk/mod.rs:727` | ✓ |
| 2 | `player_move_request` → `player_todo_clear_with_snapback` | `walk/mod.rs:682` | ✓ |
| 3 | `player_cancel_attack_and_follow` → `player_todo_clear_with_snapback` | `player/combat/mod.rs:223` | ✓ |
| 4 | `on_walk` drunk arm → `player_todo_clear_with_snapback` | `walk/mod.rs:1457` | ✓ |
| 5 | `enqueue_player_move` enqueue-time check only | `creature_todo.rs:476` | ⚠ S1 |
| 6 | `enqueue_player_use` enqueue-time check only | `creature_todo.rs:408` | ⚠ S1 |
| 7 | `finish_creature_todo_execute` stop_requested arm | `idle_stimulus.rs:3022` | ✓ |
| 8 (Use/Move/Turn) | `apply_todo_result_catch` → `send_result_player` | `creature_todo.rs:791` | ⚠ S2 |
| 8 (Go arm) | `on_walk_step_rejected` | `walk/mod.rs:1284` | ⚠ S3 |
| 9 | `finish_creature_todo_execute` stop_requested arm | `idle_stimulus.rs:3022` | ✓ |
| 10 | `player_set_attack_dest` → `player_todo_clear_with_snapback` | `player/combat/mod.rs:180` | ✓ |
| 11 | `player_stop_auto_walk` not-locked arm | `walk/mod.rs:866` | ✓ |
| 12 | `apply_todo_result_catch(ThereIsNoWay)` | `creature_todo.rs:791` | ⚠ S4 |
| 13 | monster-only (`monster_on_creature_move`) | `monster_events.rs:136` | ✗ S5 |
| 14 | (folded into site 8 — `send_result_player`) | `creature_todo.rs:816` | ⚠ S2 |

## Findings

### S1 (M): Missing executor-side `ObjectInRange` re-check + snapback (sites 5, 6)

**C++** (`cract.cc:738-757` `Use`, `cract.cc:630-646` `Move`): the executor re-checks
`ObjectInRange(1)` at **execution time**. If the target moved out of range between
enqueue and execute, it does `ToDoClear + SendSnapback` (player) + `ToDoGo` walk-to-reach
+ re-enqueue the action + `ToDoStart`.

**Rust** (`creature_todo.rs:408-461` `enqueue_player_use`, `:476-522` `enqueue_player_move`):
only checks `ObjectInRange(1)` at **enqueue time** (D6). The execute arms
(`execute_player_use` / `execute_player_move` in `idle_stimulus.rs:2914/2966`) do NOT
re-check range or send snapback — they rely on the enqueue-time check + a "needs_walk
fallback" mentioned in comments but not visible in the execute arm.

**Impact:** If the target (or player) moves between enqueue and execute, the action fires
without a snapback or walk-to-reach. The player sees "You are too far away." instead of
an automatic walk-to-reach.

### S2 (H): `send_result_player` exempt set is inverted (sites 8 + 14)

**C++**: `SendResult` SENDS a snapback for `MOVENOTPOSSIBLE` / `NOTINVITED` /
`ENTERPROTECTIONZONE` (`sending.cc:353-355`). The `Execute` catch's explicit
`SendSnapback` SKIPS these 3 to avoid a double snapback (`cract.cc:882-884`).
Net: **1 snapback** for these 3 results.

**Rust** (`creature_todo.rs:816-841`):
```rust
self.send_cancel_message(conn, rv);   // message only — NO snapback (unlike C++ SendResult)
let snapback_exempt = matches!(rv,
    PlayerIsNotInvited | ActionNotPermittedInProtectionZone | ThereIsNoWay);
if snapback && !snapback_exempt {     // snapback = had_pending_go
    self.enqueue_encoded(conn, encode_cancel_walk(...));
}
```
Rust treats the 3 as **exempt from snapback entirely** — `send_cancel_message` sends no
snapback (unlike C++ `SendResult`), and the exempt guard skips the explicit one.
Net: **0 snapback** for these 3 results.

**Impact:** A player denied entry to a house (`NOTINVITED`), blocked from a protection
zone (`ENTERPROTECTIONZONE`), or hitting an impossible move (`MOVENOTPOSSIBLE`) via a
Use/Move/Turn executor gets a failure message but **no snapback**. The client may not
resync its local position. The Go arm (`on_walk_step_rejected`) is unaffected because it
always sends snapback.

**Fix:** `send_result_player` should send the snapback for the 3 exempt results
(unconditionally, matching `SendResult`), and only gate the **non-exempt** snapback on
`had_pending_go`. Or: fold the `SendResult` snapback into `send_cancel_message` for the
3 results, keeping the exempt guard for the explicit one.

### S3 (L): `on_walk_step_rejected` always sends snapback (Go arm, site 8)

**C++** `Execute` catch for a `Go` that throws:
- `SnapbackNecessary = ToDoClear() || Stop` — true iff remaining unexecuted Go entries.
- `NOTACCESSIBLE` (adjacency fail, `cract.cc:388`): not in the 3 → explicit snapback
  **only if `SnapbackNecessary`** (remaining Gos).
- `MOVENOTPOSSIBLE` (blocked move, `cract.cc:435`): in the 3 → `SendResult` sends
  snapback; Execute catch skips.

**Rust** (`walk/mod.rs:1284-1297`): `on_walk_step_rejected` **always** sends
message + snapback, regardless of:
- Whether there are remaining Go steps (`SnapbackNecessary`).
- The result variant (no exempt set — uses `ReturnValue::NotPossible` for both
  `NOTACCESSIBLE` and `MOVENOTPOSSIBLE`).

**Impact:** For a **single-step** walk that fails the adjacency check (`NOTACCESSIBLE`
after a push, no remaining Gos), C++ sends 0 snapback but Rust sends 1. Extra snapback
is harmless (client resyncs to same position) but diverges from reference. For
multi-step autowalk (remaining Gos exist), both send 1 — match.

The doc comment at `walk/mod.rs:1280-1283` acknowledges the always-send choice but
mis-reasons that it "matches the `NOTACCESSIBLE` case" — it only matches when
`SnapbackNecessary` is true.

### S4 (M): `NOWAY` (pathfinding fail) missing snapback (site 12)

**C++** (`cract.cc:1100-1104`): when `TShortway::Calculate` fails in `ToDoGo`:
```cpp
this->ToDoClear();
if(this->Type == PLAYER){
    SendSnapback(this->Connection);   // ← UNCONDITIONAL for player
}
throw NOWAY;
```
Then the `Execute` catch sends `SendResult(NOWAY)` (message, no snapback — `NOWAY` not
in the 3). Net: **1 message + 1 snapback** (unconditional).

**Rust** (`walk_action.rs:44-45` returns `Err(ThereIsNoWay)` → caller invokes
`apply_todo_result_catch(cid, ThereIsNoWay)`): `ThereIsNoWay` is in the exempt set
(`creature_todo.rs:831`) → **no snapback**. `had_pending_go` gate also applies.
Net: **1 message + 0 snapback**.

**Impact:** A player whose walk-to-reach pathfinding fails (no path to target) gets
"There is no way." but no snapback. The client may not resync if it had started a local
walk animation.

### S5 (M): Missing player `CreatureMoveStimulus` snapback (site 13)

**C++** (`crmain.cc:920-965`): when a player's attack target moves out of range
(`CHASE_MODE_CLOSE`, target distance > 1, next entry is `TDAttack`):
```cpp
if(this->ToDoClear() && this->Type == PLAYER){
    SendSnapback(this->Connection);
}
this->ToDoWait(200);
this->ToDoAttack();
this->ToDoStart();
```

**Rust**: `monster_on_creature_move` (`monster_events.rs:136`) handles monsters only.
There is no player `CreatureMoveStimulus` — the player's attack is not re-armed when the
target moves, and no snapback is sent.

**Impact:** A player attacking a creature that walks away does not get a snapback or
attack re-arm from the move stimulus. (This is part of the broader missing player
follow/attack implementation — see `tasks/player-walk-audit.md` BUG P3.)

### S6 (L): `ReturnValue` conflation — `ThereIsNoWay` maps to both `NOWAY` and `MOVENOTPOSSIBLE`

The exempt set comment (`creature_todo.rs:790`) says `ThereIsNoWay` = `MOVENOTPOSSIBLE (52)`.
But the codebase uses `ThereIsNoWay` for C++ `NOWAY` ("There is no way.") in
`walk_action.rs:45` and the `ThereIsNoWay` description is "There is no way." — matching
`NOWAY`, not `MOVENOTPOSSIBLE` ("Sorry, not possible.").

Meanwhile `MOVENOTPOSSIBLE` from the Go arm maps to `ReturnValue::NotPossible` ("Sorry,
not possible.") in `walk_tile.rs`.

This conflation means:
- The exempt set's `ThereIsNoWay` never matches an actual `MOVENOTPOSSIBLE` (which arrives
  as `NotPossible`) — it only matches `NOWAY`.
- `NOWAY` is wrongly exempted (S4) — C++ sends unconditional snapback for it.
- `MOVENOTPOSSIBLE` arriving as `NotPossible` is never exempted — but it only reaches
  `apply_todo_result_catch` from Use/Move/Turn arms, not the Go arm (which uses
  `on_walk_step_rejected`).

**Fix:** Split the mapping: add a distinct `ReturnValue` variant for `MOVENOTPOSSIBLE`
(or use `NotPossible` consistently), and remove `NOWAY`/`ThereIsNoWay` from the exempt
set (C++ sends unconditional snapback for `NOWAY` via `ToDoGo`, not via `SendResult`).

## Summary

| ID | Sev | Finding | Status |
|----|-----|---------|--------|
| S1 | M | Missing executor-side `ObjectInRange` re-check + snapback (Use/Move arms) | Open (feature gap) |
| S2 | H | `send_result_player` exempt set inverted — 3 results get 0 snapback instead of 1 | **FIXED** |
| S3 | L | `on_walk_step_rejected` always sends snapback — no `SnapbackNecessary` check | Accepted divergence (can't distinguish NOTACCESSIBLE from MOVENOTPOSSIBLE) |
| S4 | M | `NOWAY` pathfinding fail missing unconditional snapback | **FIXED** |
| S5 | M | Missing player `CreatureMoveStimulus` combat-move-rearm snapback | Open (feature gap) |
| S6 | L | `ReturnValue::ThereIsNoWay` conflates `NOWAY` and `MOVENOTPOSSIBLE` | **FIXED** (removed from exempt set) |

**Wire parity: ✓** (opcode `0xB5` + direction byte matches exactly).
**Logic parity: ✓ for snapback conditions** (S2/S4/S6 fixed; S3 accepted as harmless extra-snapback divergence; S1/S5 are broader feature gaps).
