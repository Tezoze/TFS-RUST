# 772 Walking Parity Audit — Player Walk + Auto-Walk (Map Click)

Audit of the Rust walk engine (`crates/tfs-rust-core/src/walk/`, `creature_todo.rs`,
`walk_action.rs`, `game_loop.rs`, `tfs-rust-net/src/game_parse.rs`) against the 772
decompile (`reference/cipsoft-772/tibia-game-master/src/` — `cract.cc`, `crmain.cc`,
`crplayer.cc`, `receiving.cc`, `sending.cc`) and the TVP wire reference
(`reference/tvp-772/gameserver/src/protocolgame.cpp`).

Scope: player single-step walk (`CGoDirection`), map-click auto-walk (`CGoPath`),
stop (`CGoStop`), ToDo queue semantics, step timing/beat, blocked-step behavior,
walk-to-act, and pathfinding used by player walks. Monster-specific AI walking is out
of scope (covered by prior monster-AI audits).

Severity: **H** = observable behavior/latency wrong vs reference, **M** = divergence in
edge cases or secondary mechanics, **L** = cosmetic / unlikely-to-observe / wire nit.

---

## Summary table

| # | Sev | Area | Finding |
|---|-----|------|---------|
| 1 | H | First-step scheduling | **FIXED** — Fresh walk from standstill waited a full step duration before the first step; C++ executes it on the next beat (≤ `Beat` ms). Fix: `todo_start_go_delay` arms `server_ms + 1` (C++ `ToDoStart` clamp) when `earliest_walk_server_ms` has elapsed, instead of `get_step_duration(...)`. Removed dead `todo_go_beat_delay_ms`. Regression tests: `test_audit1_first_step_from_standstill_arms_at_one_ms`, `test_audit1_cooldown_active_arms_at_earliest_walk_time`. |
| 2 | H | `CGoStop` | **FIXED** — Stop never sent snapback and cancelled the in-flight step; C++ always snapbacks and lets the in-flight step land. Fix: `player_stop_auto_walk` now mirrors `ToDoStop` (`cract.cc:1002-1008`): if a walk is in progress (wakeup armed / `Go` queued), set `todo_stop` flag (deferred — in-flight step lands, then `finish_creature_todo_execute` does `ToDoClear + SendSnapback` per `cract.cc:891-897`); if not walking, send immediate `SendSnapback` (`cract.cc:1005-1006`). Also fixed `player_cancel_attack_and_follow` to send snapback via `player_todo_clear_with_snapback` (C++ `CCancelAttack`: `if(ToDoClear()) SendSnapback`, `receiving.cc:1339-1341`) and corrected its inverted doc comment. Regression tests: `test_phase1_player_stop_auto_walk_clears_todo`, `test_audit2_stop_from_standstill_sends_immediate_snapback`, `test_audit2_cancel_attack_sends_snapback_when_walk_pending`. |
| 3 | H | Walk-to-act | **FIXED** — `player_auto_walk_path` / `player_stop_auto_walk` didn't clear pending `walk_action`; stale deferred Use/Move fired after an unrelated walk. Fix: `player_todo_clear` now calls `clear_player_walk_action` (C++ `ToDoClear` wipes all pending entries including queued `TDUse`/`TDMove`, `cract.cc:953-989`), covering `CGoPath`/`CGoStop`/`CCancelAttack`/drunk-stagger. `try_walk_to_and_action` reordered to set the new `walk_action` **after** `player_auto_walk_path`'s internal clear. Regression tests: `test_audit3_auto_walk_clears_stale_walk_action`, `test_audit3_stop_clears_stale_walk_action`, `test_audit3_walk_to_use_preserves_walk_action`. |
| 4 | M | Queue model | **FIXED** — Rust queued relative `Direction`s; C++ `TDGo` stores absolute coordinates (`receiving.cc:141-160`), so a mid-walk push left the rest of the path silently replayed offset by the push delta instead of aborting. Fix: added `walk_destinations: VecDeque<Position>` overlay on `CreatureBase`, populated in `player_move_request` / `player_auto_walk_path` / `player_combat.rs` chase path alongside `walk_queue`. `on_walk` pops the parallel destination and verifies adjacency (`cract.cc:386-389`: `Distance > 1 || OrigZ != DestZ → NOTACCESSIBLE`) before the step; on failure, `on_walk_step_rejected` sends `SendResult("Sorry, not possible.")` + `SendSnapback` + `ToDoClear` + `ToDoYield` (`cract.cc:870-889`). Regression tests: `test_audit4_push_mid_auto_walk_aborts_remaining_path`, `test_audit4_unpushed_auto_walk_completes_normally`. |
| 5 | M | Floor-change cooldown | **FIXED** — `last_step_cost = 2` on z-change doubled the post-stair-hop walk delay on the LinearGo path; C++ `NotifyGo` never applies a z multiplier. Fix: `completed_step_duration_ms` LinearGo arm maps `last_step_cost` to the C++ `NotifyGo` waypoint cost (3 if diagonal same-z, else 1) instead of `last_step_cost.max(1)` (`cract.cc:1526-1528`). Regression test: `linear_go_completed_step_zchange_uses_one_waypoint_cost`. |
| 6 | M | Walk delay source | **FIXED** — Two sources of truth for the walk cooldown (`earliest_walk_server_ms` vs recomputed `get_walk_delay_logical`) could disagree, e.g. after mid-cooldown speed change. Fix: `on_walk` beat-path gate now derives from `earliest_walk_server_ms` directly (C++ single source `EarliestWalkTime`, `cract.cc:918-923`/`:1515-1525`), not from `get_walk_delay_logical` which recomputed `completed_step_duration_ms` from current speed. Regression test: `test_audit6_on_walk_gate_uses_earliest_walk_time`. |
| 7 | M | Player checks | `tile_query_add_player` lacks PZ-lock (`ENTERPROTECTIONZONE`) and house-invite (`NOTINVITED`) checks (partially documented as unported) |
| 8 | M | Auto-climb | Stair-hop uses TFS 1098 `hasHeight(3)` / `queryDestination` semantics, not 772 `GetHeight >= 24` climb-on-blocked semantics |
| 9 | M | Pathfinding radius | Reverse-path viewport is always 10; C++ `TShortway` uses `VisibleX/Y = 7` for players |
| 10 | M | Walk-to-act delay | `WALK_ACTION_DELAY_MS = 400` (TFS) on the 772 path; C++ uses `ToDoWait(100)` (Use/Move) or `1000` (creature push) |
| 11 | L | Blocked-step message class | Failure text sent as message class 21 (`TALK_STATUS_MESSAGE`); C++ `SendResult` uses 23 (`TALK_FAILURE_MESSAGE`) |
| 12 | L | AutoWalk parsing | Invalid direction byte rejects the whole packet; C++ `continue`s past the byte and keeps the rest of the path |
| 13 | L | Drunk stagger | Rust staggers players only; C++ `TCreature::Go` staggers any drunk creature |
| 14 | L | Same-beat chaining | C++ `Execute` drains consecutive zero-delay ToDo entries in one wakeup; Rust re-arms at `+1 ms` (lands next beat) in several paths |
| 15 | L | ToDoGo dedup | C++ dedups a repeated identical trailing `TDGo` (`throw NOERROR`); Rust has no equivalent |
| 16 | L | `NotifyGo` no-bank case | C++ leaves `EarliestWalkTime` untouched when the dest tile has no BANK item; Rust defaults ground speed to 150 |

---

## Detailed findings

### 1. [H] First step from standstill is delayed by a full step duration

C++ `TCreature::CalculateDelay` for `TDGo` (`cract.cc:918–924`):

```cpp
case TDGo:{
    if(this->EarliestWalkTime > ServerMilliseconds){
        Delay = this->EarliestWalkTime - ServerMilliseconds;
    }
    break; // else Delay stays 0
}
```

With `ToDoStart` clamping `Delay < 1` to `1` (`cract.cc:1016`), a player who has been
standing (i.e. `EarliestWalkTime` already passed) gets `NextWakeup = now + 1` — the
step executes on the **next beat drain** (≤ 200 ms, avg ~100 ms). Only *subsequent*
steps wait out `EarliestWalkTime` set by `NotifyGo`.

Rust (`walk/mod.rs:486–508` `todo_start_go_delay` + `:466–480 todo_go_beat_delay_ms`):

```rust
let calc_delay = if earliest > server_ms {
    earliest - server_ms
} else {
    self.todo_go_beat_delay_ms(cid)   // = get_step_duration(...) — a FULL step duration
};
```

When `earliest_walk_server_ms` has passed (fresh walk), the fallback arms the wakeup a
full quantized step duration in the future (e.g. GoStrength 220 on 150-waypoint ground:
`150×1000 / 520 = 288 ms → ceil(beat 200) = 400 ms`) instead of 1 ms. Every walk
started from rest — both `player_move_request` and `player_auto_walk_path` — carries up
to one extra step of input latency vs the reference. (`idle_stimulus_tests.rs:4120–4125`
works around this by zeroing `earliest_walk_server_ms`.)

The doc comment on `todo_go_beat_delay_ms` (“one beat from server_ms (`cract.cc:912`)”)
does not match either its implementation (full step duration) or the C++ (0 → clamp 1).

**Fix direction:** when `earliest_walk_server_ms <= server_ms`, arm the wakeup at
`server_ms + 1` (C++ clamp), not at `+ step_duration`. The `on_walk` →
`get_walk_delay_logical` re-check already protects against early execution.

### 2. [H] `CGoStop` / `ToDoStop`: missing snapback + premature cancel of the in-flight step — **FIXED**

C++ (`cract.cc:1002–1008`, Execute loop `cract.cc:795–806`, `:891–897`):

```cpp
void TCreature::ToDoStop(void){
    if(this->LockToDo){
        this->Stop = true;                 // deferred: current entry still lands
    }else if(this->Type == PLAYER){
        SendSnapback(this->Connection);    // immediate snapback
    }
}
```

- Not locked → **immediate** `SendSnapback` (0xB5).
- Locked → `Stop` flag; on the next `Execute` wake the pending entry either executes
  first (delay elapsed) and *then* `ToDoClear + SendSnapback`, or (delay pending)
  `ToDoClear + SendSnapback` without executing. Either way the client **always** gets a
  snapback so it stops its local walk animation cleanly.

Rust (`walk/mod.rs:869–879` `player_stop_auto_walk`, beat path):

```rust
// 772 `ToDoStop` — clears the queue without snapback (`receiving.cc:201-211`).
self.player_todo_clear(cid);
```

- **No snapback is ever sent** — the code comment (“CGoStop doesn't send one”) is
  wrong; `receiving.cc` delegates to `ToDoStop`, which does.
- The queue is cleared immediately, so a step whose wakeup is already armed is
  cancelled; in C++ under `LockToDo` with delay elapsed the in-flight step still lands
  before the clear.

Same inverted claim in `player_combat.rs:178–189` (`player_cancel_attack_and_follow`
doc: “snapback only when LockToDo … otherwise it just clears the queue” — the C++ is
the opposite: not-locked is the *immediate snapback* branch).

### 3. [H] Stale `walk_action` survives a new auto-walk / stop — **FIXED**

C++ `CGoPath` / `CGoDirection` open with `ToDoClear()` (`receiving.cc:120–199`), which
wipes **all** pending entries — including a queued `TDUse` / `TDMove` from a prior
walk-to-use. `ToDoStop` ends in the same clear.

Rust: `player_move_request` calls `clear_player_walk_action` (`walk/mod.rs:711`), but:

- `player_auto_walk_path` (`walk/mod.rs:756–806`) never clears `walk_action` /
  `walk_action_due`;
- `player_stop_auto_walk` → `player_todo_clear` (`walk/mod.rs:659–674`) clears
  `todo.queue` / `walk_queue` but not `walk_action`.

Repro: click a distant item (Use) → while walking, map-click somewhere else → when the
*new* walk completes, `on_player_walk_complete` (`walk_action.rs:40–57`) schedules the
**old** Use action, which then executes from the wrong position (or gets `defer`red
forever). On 1098 the `cancel_next_walk` path does call `clear_player_walk_action`
(`walk/mod.rs:1535`), so this is a 772-path-only hole.

### 4. [M] Relative-direction queue vs absolute-coordinate `TDGo` — **FIXED**

C++ `CGoPath` accumulates **absolute** coordinates per step at packet-receive time
(`receiving.cc:141–160`) and `TCreature::Go` throws `NOTACCESSIBLE` when the stored
destination is no longer adjacent (`cract.cc:386–389`):

```cpp
int Distance = max(abs(OrigX - DestX), abs(OrigY - DestY));
if(Distance > 1 || OrigZ != DestZ) throw NOTACCESSIBLE;
```

If the player is pushed one tile mid-auto-walk, the next `TDGo` is now 2 tiles away →
walk aborts with `SendResult` + snapback.

Rust stored `Direction`s in `walk_queue` and applied them from wherever the player
currently stands. After a push, the rest of the path was silently replayed **offset by
the push delta**, walking the player to the wrong destination instead of aborting.

**Fix:** added a `walk_destinations: VecDeque<Position>` overlay on `CreatureBase`,
populated alongside `walk_queue` in `player_move_request` / `player_auto_walk_path` /
`player_combat.rs` chase path. The overlay stores the absolute destination of each step
(C++ `TDGo` semantics). `on_walk` pops the parallel destination and checks adjacency
before the step — if `max(|dx|, |dy|) > 1 || z differs`, it calls `on_walk_step_rejected`
(`ReturnValue::NotPossible`) which sends `SendResult("Sorry, not possible.")` +
`SendSnapback` + `ToDoClear` + `ToDoYield` (`cract.cc:870-889`). The `Err(ret)` branch
of `internal_move_creature_step` was refactored into the same `on_walk_step_rejected`
helper to share the error-handling path. The `walk_destinations` queue uses `push_front`
in execution order so `pop_back` stays in sync with `walk_queue`'s LIFO-at-back.

### 5. [M] Stair-hop doubles the next-step delay (`last_step_cost = 2` on LinearGo path) — **FIXED**

`last_step_cost_for_move` returns `2` for any z-change (`walk_timing.rs:320–333`, a TFS
1098 `lastStepCost` rule), and `completed_step_duration_ms` on the **LinearGo** path
multiplies waypoints by it (`walk_timing.rs:254–269`):

```rust
StepSpeedModel::LinearGo =>
    linear_go_step_duration_ms(kind, base, ground_speed, base.last_step_cost.max(1), mech),
```

C++ `NotifyGo` (`cract.cc:1526–1534`) only multiplies waypoints ×3 for a **diagonal
same-z** move (`DestX != OrigX && DestY != OrigY && DestZ == OrigZ`); a stair-hop /
floor change gets ×1. So after climbing, `get_walk_delay_logical` holds the player for
2× the correct duration. Note the *other* cooldown source (`earliest_walk_server_ms`,
set from `get_step_duration_ms_with_direction(dir)` at `walk/mod.rs:1438–1463`) uses the
direction-based cost (×1 for a cardinal climb) — i.e. the two mechanisms disagree with
each other, and only the direction-based one matches C++.

### 6. [M] Two sources of truth for the walk cooldown — **FIXED**

C++ has exactly one: `EarliestWalkTime`, fixed by `NotifyGo` when the step lands, and
consumed by `CalculateDelay`. Rust keeps both:

- `earliest_walk_server_ms` (`walk/mod.rs:1459–1463`) — used by `todo_start_go_delay`;
- a recomputation in `get_walk_delay_logical` → `completed_step_duration_ms`
  (`walk_timing.rs:353–368`) — used by `on_walk` at wake time.

Besides finding 5, the recomputation reads **current** speed/conditions: casting haste
or getting paralyzed *between* steps changes the in-flight cooldown, whereas C++ keeps
the delay fixed at step-completion time. Recommend deriving the `on_walk` gate from the
stored `earliest_walk_server_ms` on the beat path.

### 7. [M] Missing 772 player move checks: PZ-lock and house invite

`TPlayer::MovePossible` (`crplayer.cc:363–380`) adds, when `Execute`:

- `ENTERPROTECTIONZONE` — pz-locked player (`EarliestProtectionZoneRound > RoundNr`)
  stepping from non-PZ into PZ;
- `NOTINVITED` — house tile without invite / `ENTER_HOUSES` right.

`tile_query_add_player` (`walk_tile.rs:531–607`) implements neither (no PZ flag check,
no house branch at all for players). `walk/mod.rs:11–12` documents “full PZ /
`Tile::queryAdd` … not ported”, so this is a known gap — recorded here because both
results are also snapback-relevant (`sending.cc:353–355` sends snapback for them via
`SendResult`, and `Execute` deliberately excludes them from its own snapback).

### 8. [M] Auto-climb (stair hop) follows TFS 1098, not the 772 decompile

Rust `resolve_player_move_destination` (`walk_tile.rs:115–192`) is a port of TFS
`game.cpp:797–841` (`hasHeight(3)`, speculative up/down before `queryAdd`, plus a
`queryDestination` chain loop `walk/mod.rs:1637–1679` from `tile.cpp:735–830`).

772 (`cract.cc:415–437`) is shaped differently:

- climb is attempted **only after** the plain `MovePossible(dest)` fails;
- climb-up requires `GetHeight(dest, z+? ) >= 24` (elevation units, ~3 stacked height
  items), origin/dest guards on `BANK`/`UNPASS` above, `MovePossible(..., Jump=true)`,
  and `0 < DestZ < 15` bounds;
- there is no `queryDestination`-style multi-hop chain in `Go` — ladder/rope-style
  floorchange is item/`moveuse` driven.

Outcomes will often coincide (24 elevation ≈ three height items), but blocked/edge
cases (e.g. climb attempted even when the flat move would have succeeded, chained
floorchange tiles) can diverge. Worth a targeted behavioral comparison on real 772 map
data; at minimum the C++ reference comments should stop claiming this path is 772.

### 9. [M] Player pathfinding viewport is 10; C++ uses 7 for players

`TCreature::ToDoGo` (`cract.cc:1093–1095`):

```cpp
int VisibleX = (this->Type == PLAYER) ? 7 : 10;
```

Rust `REVERSE_PATH_VIEW_RADIUS = 10` is used unconditionally (`pathfinding.rs:25`,
`:745–751` — “772 monster chase always uses VisibleX/Y = 10”). Player-initiated
server-side pathing (walk-to-use/move via `try_walk_to_and_action` →
`get_creature_path_to`) can therefore find paths to objects that the reference would
reject with `NOWAY` (“There is no way.”) beyond the 7-tile viewport, and vice-versa
skew branch-and-bound pruning. Also verify the `NOWAY` failure path: C++ `ToDoGo` does
`ToDoClear + SendSnapback + throw NOWAY` (→ `SendResult` “There is no way.”);
`try_walk_to_and_action` (`walk_action.rs:130–149`) just returns `false` — the caller
must produce the same message + snapback.

### 10. [M] Walk-to-act wait uses the TFS 400 ms constant on the 772 path

`WALK_ACTION_DELAY_MS = 400` (`walk_action.rs:17`) is TFS `createSchedulerTask(400,…)`.
772 chains the action in the ToDo queue behind the walk with `ToDoWait(Delay)` where
`Delay = 100` for Use/Move and `1000` (plus remaining walk time) when pushing a
creature (`cract.cc:1143–1162`, `:1183–1190`). Net effect: deferred use/move after a
walk fires ~300 ms later than the reference; creature pushes fire ~600 ms earlier.

### 11. [L] Blocked-step failure message uses class 21 instead of 23

`SendResult` (`sending.cc:339`, `:351–355`) sends “Sorry, not possible.” as
`TALK_FAILURE_MESSAGE = 23` (`enums.hh:674`) followed by a snapback for
`MOVENOTPOSSIBLE`/`NOTINVITED`/`ENTERPROTECTIONZONE`. Rust sends the text with
`MESSAGE_STATUS_SMALL: u8 = 21` (`walk/mod.rs:140`, `:1362–1370`) plus
`encode_cancel_walk`. The message-class byte differs on the wire (bottom-status vs
failure class); the snapback itself is correct parity here — `Execute`’s *own*
snapback exclusion for those three results exists precisely because `SendResult`
already sent one.

### 12. [L] Auto-walk packet parsing: whole-packet rejection on a bad byte

`parse_auto_walk` (`game_parse.rs:277–313`) errors on any direction byte outside 1–8,
dropping the entire packet (logged in `protocol_game.rs:130–133`). C++ `CGoPath`
(`receiving.cc:148–158` `default: continue`) skips only the invalid byte and executes
the remaining path. Same for TVP. Low impact with well-behaved clients. The `n > 128`
cap matches TVP (`protocolgame.cpp:755–758`); C++ receiving.cc itself accepts up to 255.

### 13. [L] Drunk stagger is player-only in Rust

`TCreature::Go` staggers **any** drunk creature (`cract.cc:392–413`) — monsters say
“Hicks!” with `TALK_ANIMAL_LOW`. Rust `on_walk` only checks
`Some(CreatureKind::Player(p))` (`walk/mod.rs:1305–1310`), so drunk monsters never
stagger. (Talk-mode handling for monsters already exists in `execute_talk`,
`idle_stimulus.rs:2215–2231`.)

### 14. [L] Zero-delay ToDo entries chain a beat late

C++ `Execute` is a `while(true)` loop: consecutive entries with `CalculateDelay() == 0`
run in the **same wakeup** (e.g. stagger's `TDTalk` right after the clear, walk-to-use's
`TDUse` right after the last `TDGo` once the wait elapsed). Rust executes one action per
wakeup and re-arms at `server_ms + 1` (`schedule_immediate_todo_wakeup`,
`creature_todo.rs:394–397`; drunk path `walk/mod.rs:1318–1320`), which lands on the
*next* beat — each chained action drifts +1 beat (up to 200 ms) vs the reference.

### 15. [L] Missing trailing-`TDGo` dedup

`ToDoGo` under `LockToDo` throws `NOERROR` when the last queued entry is an identical
`TDGo` (`cract.cc:1057–1066`), silently ignoring duplicate go-to requests mid-queue.
No Rust equivalent in `enqueue_creature_go_at` / chase re-arm paths (the `has_go()`
guard dedups the *action*, not the destination). Mostly affects internal callers
(chase/use), not client packets.

### 16. [L] `NotifyGo` without a BANK item leaves the cooldown unchanged

C++ scans the dest tile for the first `BANK` object and, if none exists, **skips** the
`EarliestWalkTime` update entirely (`cract.cc:1513–1535` — `if(Bank != NONE)`), i.e. no
walk cooldown from that step. Rust substitutes ground-speed 150 when the tile has no
ground (`ground_speed_for_tile_body`, `walk/mod.rs:142–150`; `gs == 0 → 150` in
`walk_timing.rs:212`). Unreachable on sane map data (a walkable tile without BANK), so
informational only.

---

## Verified parity (no action needed)

- **Speed formula** — `GetSpeed() = GoStrength*2 + 80` (`crmain.cc:477–485`) ↔
  `linear_go_effective_speed` (`formulas.rs:132`; tests: `42 → 164`, `0 → 80`).
- **Step duration** — `(Waypoints×1000)/GetSpeed`, diagonal ×3 **before** the Beat
  ceil, `ceil(Delay/Beat)*Beat` (`cract.cc:1526–1534`) ↔ `linear_go_step_duration_ms`
  + `ceil_to_walk_quantizer` (`walk_timing.rs:183–220`). Waypoints read from the
  **destination** tile of the completed step.
- **Beat** — default 200 ms (`config.cc:102`) ↔ `profile.beat_ms = 200`
  (`formulas.rs:279`); logical clock advanced in beat steps
  (`game_loop.rs:633–687` ↔ `crmain.cc:1142–1157` `MoveCreatures`).
- **New-walk preamble** — `if(ToDoClear()) SendSnapback` on both `CGoDirection` and
  `CGoPath` (`receiving.cc:120–199`) ↔ `player_todo_clear_with_snapback`
  (`walk/mod.rs:676–693`), including `ToDoStart`'s `Delay < 1 → 1` clamp
  (`todo_start_from_action`, `creature_todo.rs:237–243`).
- **Packet decode** — 0x64 length/dir validation vs TVP (`protocolgame.cpp:755–788`),
  1–8 direction mapping, execution order (reverse + back-pop = client order), 0x69 stop
  routing, 0x65–0x68/0x6A single-step opcodes.
- **Blocked-step result** — message + snapback on `MOVENOTPOSSIBLE` matches
  `SendResult` (`sending.cc:351–355`); queue cleared + idle stimulus ≈ `ToDoClear` +
  `ToDoYield` → `IdleStimulus` (`cract.cc:870–889`), modulo the message-class nit (#11).
- **Drunk stagger math** — `StaggerChance = max(7 − DrunkLevel, 1)`, `rand() %
  StaggerChance == 0`, random cardinal, clear + snapback + “Hicks!”
  (`cract.cc:392–413` ↔ `walk/mod.rs:114–137`, `:1300–1321`) — for players.
- **TShortway shape** — reverse dest→origin search, `Waylength + Waypoints +
  MinWaypoints×(Distance−1)` heuristic, diagonal `+Waypoints×2` (total ×3),
  branch-and-bound pruning (`cract.cc:8–310` ↔ `pathfinding.rs`).

---

## Suggested fix order

1. **#1** first-step delay (`todo_go_beat_delay_ms` → `1 ms` clamp) — biggest felt
   difference; touches every walk.
2. **#2** stop snapback + deferred-clear semantics (`Stop` flag model or
   execute-then-clear at the armed wakeup).
3. **#3** clear `walk_action` in `player_auto_walk_path` / `player_todo_clear`.
4. **#5/#6** unify the cooldown on `earliest_walk_server_ms`; drop `last_step_cost`
   from the LinearGo completed-step math.
5. **#4** absolute-destination walk queue (or a per-step origin check that aborts with
   `NOTACCESSIBLE` semantics when the expected origin doesn't match).
6. **#7–#10** in whatever order PZ/house work is scheduled; #9 is a one-line radius
   parameterization plus `NOWAY` result plumbing.
7. **#11–#16** opportunistically.

Each fix should land with a regression test mirroring the C++ reference (see
`idle_stimulus_tests.rs` walk tests for the harness pattern).
