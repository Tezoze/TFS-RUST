# F8 Sub-Plan — Route Player Actions Through the ToDo `Execute` Engine

**Parent:** `tasks/unified-beat-engine-phases.md` Phase 0 / Finding **F8**.
**Audit source:** `docs/GAME_LOOP_772_AUDIT.md` Finding 8.
**Decompile ref tree:** `reference/cipsoft-772/tibia-game-master/src/` — `receiving.cc`,
`cract.cc`, `cr.hh`. **(772 mechanics = `tibia-game-master/src/`; do not cite `gameserver/src/`
or repo-root `src/` for this ToDo model.)**
**Status:** ⬜ NOT STARTED.

## 0. Objective

Make player non-walk actions flow through the **same** ToDo `Execute` pipeline that 772 walk +
monster AI already use, exactly as the decompile does: command handler → `ToDo*` builder →
`ToDoStart` → drained in `MoveCreatures`/`Execute` at beat time, gated by per-action
`CalculateDelay`. This closes the last structural divergence (`Finding 8`) so the ToDo engine is
literally the sole action scheduler — the precondition for making it the single engine across
eras.

## 1. Scope correction (mirror the decompile, don't over-route)

Finding 8 grouped `player_look_at` with the ToDo-routing gap. **That is wrong per the
decompile** and this plan corrects it:

| Client command | 772 handling | Route through ToDo? |
|---|---|---|
| `CUseObject` (single) `receiving.cc:384` | `ToDoWait(100)` + `ToDoUse(1,…)` + `ToDoStart` | **Yes** |
| `CUseTwoObjects` `receiving.cc:430` | `ToDoWait(100)` + `ToDoUse(2,…)` + `ToDoStart` | **Yes** |
| `CUseOnCreature` `receiving.cc:~520` | `ToDoWait(100)` + `ToDoUse(2, Obj, CrObj)` + `ToDoStart` | **Yes** |
| `CMoveObject` `receiving.cc:233` | `ToDoMove(…)` + `ToDoStart` (no wait) | **Yes** |
| `CTradeObject` `receiving.cc:290` | `ToDoTrade(…)` + `ToDoStart` (no wait) | **Yes** |
| `CTurn` (rotate an object) `receiving.cc:~560` | `ToDoWait(100)` + `ToDoTurn(…)` + `ToDoStart` | **Yes** |
| `CTalk` (say/yell/whisper) `receiving.cc:750` | `ToDoTalk(…)` + `ToDoStart` (no wait) | **Yes** (local speech modes) |
| `CLookAtPoint` `receiving.cc:717` | `Look(ID, Obj)` **immediately**; no ToDo entry | **No — stays reactive** |
| `CInspectTrade` `receiving.cc:337`, `UpdateContainer`/`BrowseField` refresh reads | immediate reads | **No — stays reactive** |

There is **no `TDLook`** in `TToDoEntry` (`cr.hh:461-513`). Read-only queries (look, inspect
trade, container refresh) are answered inline in `receiving.cc` and must stay reactive. Channel
speech / rule-violation talk modes in `CTalk` are also handled inline — only **local speech**
(`TALK_SAY`/yell/whisper) goes through `ToDoTalk`.

## 2. The decompile model (authoritative)

### 2.1 The `TToDoEntry` union (`cr.hh:461-513`)
Variants: `Wait{Time}`, `Go{x,y,z}`, `Rotate{Direction}`, `Move{Obj,x,y,z,Count}`,
`Trade{Obj,Partner}`, `Use{Obj1,Obj2,Dummy}`, `Turn{Obj}`,
`Talk{Text,Mode,Addressee,CheckSpamming}`, `ChangeState{NewState}`.

### 2.2 Command → builder → start (`receiving.cc`)
Each handler resolves nothing itself except reading coords/type/RNum, then calls the `ToDo*`
builder (which resolves the `Object` via `GetObject(ID,x,y,z,RNum,Type)` and **throws `RESULT`**
on failure), then a single `ToDoStart`. Handlers that touch objects the player must "reach"
prepend `ToDoWait(100)` (use / turn), giving a 100 ms floor before execution.

### 2.3 `ToDoStart` (`cract.cc:955-1024`)
`if NrToDo != 0 → LockToDo=true; ActToDo=0; Delay=CalculateDelay(); Delay=max(Delay,1);
NextWakeup=ServerMilliseconds+Delay; ToDoQueue.insert(NextWakeup, ID)`. The `+1` clamp guarantees
forward progress (already honored in Rust `todo_start_from_action`).

### 2.4 `CalculateDelay` — per-action gate (`cract.cc:901-960`)
| Action | Delay source |
|--------|--------------|
| `TDWait` | `max(Wait.Time, EarliestWalkTime) − ServerMs` |
| `TDGo` | `EarliestWalkTime − ServerMs` |
| `TDUse` | `EarliestMultiuseTime − ServerMs` **only if `Obj2 != 0`** (two-object); single-object use is ungated |
| `TDAttack` | `max(EarliestAttackTime, EarliestSpellTime) − ServerMs` |
| `TDMove`, `TDTrade`, `TDTurn`, `TDTalk`, `TDRotate`, `TDChangeState` | **0** (run immediately in the drain) |

### 2.5 `Execute` drain (`cract.cc:783-898`)
`while(true)`: break if `!LockToDo || IsDead || NextWakeup > ServerMs`; if `ActToDo >= NrToDo` →
`ToDoClear` + `IdleStimulus`; if `CalculateDelay > 0` → schedule (or `ToDoClear`+snapback if
`Stop`) and break; else pop entry, `ActToDo+=1`, dispatch by `Code`. On thrown `RESULT`:
`ToDoClear`/`Stop`; `EXHAUSTED → ToDoWait(1000)+ToDoStart`, else `ToDoYield`; player →
`SendResult` + conditional `SendSnapback` (skip for `MOVENOTPOSSIBLE`/`NOTINVITED`/
`ENTERPROTECTIONZONE`).

### 2.6 Executors and reach-to-object
- `Use` (`cract.cc:~600-760`): if the target isn't reachable, it prepends `ToDoGo(dest)` +
  re-enqueues `ToDoMove`/`ToDoUse` + `ToDoStart` and returns — i.e. **walk-to-use is a Go entry
  in front of the action**, not a special path. Two-object use sets
  `EarliestMultiuseTime = ServerMs + 1000` (`cract.cc:765`).
- `Move`/`Trade`/`Turn` executors re-validate the object at execute time (`Obj.exists()` →
  `NOTACCESSIBLE`).

## 3. Current Rust state

- `CreatureAction` (`creature_todo.rs:87`) has only `Go`, `Wait{delay_ms}`, `Attack`,
  `Talk{text}`, `Rotate{target_id}`. **Missing: `Use`, `Move`, `Trade`, `Turn`.**
- `execute_creature_todo_action` (`idle_stimulus.rs:2227`) dispatches Go/Wait/Talk/Attack(/Rotate).
- Reactive handlers dispatched inline in `game_loop.rs` `handle_game_packet`:
  `player_use_item` / `player_use_item_ex` (`container_ui.rs:503/567`), `player_look_at`
  (`game_world_inventory.rs:900`), container ops, `Say`.
- Mitigations already present (keep as fallback until replaced):
  - `game_packet_requires_timed_action` (`game_loop.rs:125`) + `player_packet_action_ready`
    (`game_world.rs:175`) approximate the gate.
  - Walk-to-use already deferred: `walk_action.rs` + `try_run_player_walk_action_from_todo`.
  - `player_use_item_ex_ready` already reads `EarliestMultiuseTime` (`game_world.rs:228`).

## 4. Target Rust design

Extend the existing ToDo machinery — do **not** invent a parallel player-action queue.

1. **Extend `CreatureAction`** (`creature_todo.rs`) with object-carrying variants. Store
   **resolved-at-enqueue identity** (not raw wire coords) so execute-time re-validation is a
   simple lookup:
   ```rust
   enum CreatureAction {
       Go, Wait { delay_ms }, Attack, Talk { text }, Rotate { target_id }, // existing
       Use { obj1: ActionObjectRef, obj2: Option<ActionObjectRef>, open_index: u8 },
       Move { obj: ActionObjectRef, dest: MoveDest, count: u16 },
       Trade { obj: ActionObjectRef, partner: CreatureId },
       Turn { obj: ActionObjectRef },
   }
   ```
   `ActionObjectRef` = the Rust analog of `GetObject` result: enough to re-locate the item at
   execute time (map `Position+stackpos` **or** `Inventory{cid,slot}` **or** `Container{cid,slot}`
   + expected item type for validation). This mirrors the decompile's `Object` handle resolved in
   the `ToDo*` builder.
2. **Add `ToDo*` builders** (`creature_todo.rs`, alongside `enqueue_creature_go/wait/talk`):
   `enqueue_player_use`, `enqueue_player_move`, `enqueue_player_trade`, `enqueue_player_turn`.
   Each **resolves the object now** (via the existing inventory/tile query helpers) and returns
   `Err(ReturnValue)` if resolution fails — mirroring the builder `throw RESULT` at
   `receiving.cc` enqueue time. Use/Turn builders also `enqueue_creature_wait(100)` first.
3. **`CalculateDelay` arms** — the delay is computed in `todo_start_from_action` /
   `execute_creature_todo_action` today. Add:
   - `Use` with `obj2.is_some()` → gate on `earliest_multiuse_server_ms` (field already exists,
     `creature/base.rs:125`).
   - `Move`/`Trade`/`Turn` → delay 0 (execute in-drain).
   Keep single-object `Use` ungated (matches `Obj2 == 0`).
4. **`Execute` dispatch arms** in `execute_creature_todo_action` (`idle_stimulus.rs`): route
   `Use → player_use_item(_ex)`, `Move → player_move_object`, `Trade → player_trade`,
   `Turn → player_rotate_object`, **re-validating the object** first (`NOTACCESSIBLE` on failure).
   Reuse the existing `player_use_item` / `player_use_item_ex` bodies as the executor — they are
   *moved*, not rewritten. On the executor returning a `ReturnValue` error, run the C++ catch:
   `EXHAUSTED → ToDoWait(1000)+ToDoStart`; else `ToDoYield`; emit `SendResult` + conditional
   snapback. (`player_execute_attack` already models this catch — mirror it.)
5. **Set `EarliestMultiuseTime`** on successful two-object use:
   `earliest_multiuse_server_ms = server_ms + 1000` (`player_apply_multiuse_exhaust` already
   exists, `game_world.rs:240` — call it from the Use executor instead of the reactive path).
6. **Walk-to-reach** = prepend `Go` + re-enqueue the action, exactly like the `Use` executor.
   Fold the existing `walk_action` deferral into this (`Move`/`Use` when not adjacent enqueue a
   `Go` front + themselves, then `ToDoStart`).
7. **Reroute `handle_game_packet`** (`game_loop.rs`): `UseItem`/`UseItemEx`/`MoveObject`/
   `Trade`/`Turn`/`Say`(local) arms call the `enqueue_*` builder + `todo_start_from_action`
   instead of the executor directly. On builder `Err(rv)` → `SendResult(rv)` (matches the
   handler-level `catch(RESULT r)` in `receiving.cc`). **Leave `LookAt`, `InspectTrade`, container
   refresh, and channel-talk arms reactive.**

## 5. Delay / gate parity table (target)

| CreatureAction | Prepend `Wait{100}` | CalculateDelay gate | Executor | Sets exhaustion |
|---|---|---|---|---|
| `Use{obj2:None}` | yes | none | `player_use_item` | — |
| `Use{obj2:Some}` | yes | `earliest_multiuse_server_ms` | `player_use_item_ex` | `+1000 ms` multiuse |
| `Move` | no | none (0) | `player_move_object` | — |
| `Trade` | no | none (0) | `player_trade` | — |
| `Turn` | yes | none (0) | `player_rotate_object` | — |
| `Talk{say/yell/whisper}` | no | none (0) | broadcast (existing) | — |

## 6. Phased steps (checklist)

Each step gates on `rtk cargo check && rtk cargo clippy --all-targets && rtk cargo test`. Keep the
reactive path alive behind the existing gate until its ToDo replacement is verified, then delete.

- [ ] **S1 — `ActionObjectRef` + enum variants.** Add `Use`/`Move`/`Trade`/`Turn` to
      `CreatureAction` + `has_*` helpers; no behavior wired yet. Compile only.
- [ ] **S2 — Builders.** `enqueue_player_use/move/trade/turn` with resolve-now +
      `Err(ReturnValue)`; Use/Turn prepend `Wait{100}`. Unit-test each builder's queue shape +
      failure `RESULT`.
- [ ] **S3 — CalculateDelay.** Wire the multiuse gate for two-object `Use`; 0 for Move/Trade/Turn.
      Test: two-object use within 1000 ms defers; single-object use does not.
- [ ] **S4 — Execute dispatch.** Move `player_use_item(_ex)` bodies into the `Use` execute arm
      (re-validate → `NOTACCESSIBLE`); add `Move`/`Trade`/`Turn` arms; wire the `RESULT` catch
      (`EXHAUSTED`→wait1000+start, else yield, `SendResult`+snapback). Set multiuse exhaustion on
      success.
- [ ] **S5 — Walk-to-reach.** Route not-adjacent Use/Move through `Go`-prepend + re-enqueue;
      retire the bespoke `walk_action_due` path in favor of the unified Go+action enqueue (verify
      `try_run_player_walk_action_from_todo` tests still describe the same outcome).
- [ ] **S6 — Reroute handlers.** Point `UseItem`/`UseItemEx`/`MoveObject`/`Trade`/`Turn`/`Say`
      arms at builder+`ToDoStart`; keep `LookAt`/`InspectTrade`/container-refresh/channel-talk
      reactive. Remove those opcodes from `game_packet_requires_timed_action` /
      `player_packet_action_ready` once the ToDo gate subsumes them.
- [ ] **S7 — Delete dead reactive paths + mitigations** that the ToDo path now covers. `grep`
      confirms no inline executor calls for the rerouted opcodes in `handle_game_packet`.
- [ ] **S8 — Tests** (see §8) + update `tasks/lessons.md`.

## 7. Risks & parity notes

- **Object identity across enqueue→execute.** The decompile resolves `Object` at enqueue and the
  executor re-checks `exists()`. Rust must do the same: resolve to `ActionObjectRef` now,
  re-validate (item still at that pos/slot, expected type) at execute; mismatch → `NOTACCESSIBLE`.
  Do **not** cache a raw `ItemId` that could be reused by the SlotMap — validate the location +
  type, matching `GetObject`.
- **Intra-beat ordering change.** Today player actions run reactively (at packet receipt); after
  F8 they run in the beat drain interleaved with other creatures' ToDo. This is the *intended*
  parity fix (Finding 8) but is observable — verify use/move feel and that reply packets still
  land in the same beat's `SendAll` flush (they will: drain → flush at beat end).
- **Single-object use is ungated.** Don't add a multiuse delay to single-object use — the
  decompile only gates `Obj2 != 0`. The `Wait{100}` floor is the only single-use delay.
- **`Say` scope.** Only local speech modes go through `ToDoTalk`. Route channel/private/rule
  modes reactively (they don't map to `TDTalk`).
- **1098 reuse.** These variants are era-neutral (they're the ToDo model). When Phase 3/4 moves
  1098 onto ToDo, the 1098 delays come from the profile (`attack_speed_ms`, multiuse gate); the
  action *plumbing* is shared. No `beat_driven_loop` branch in the new arms — read the gate from
  the same seam walk already uses.
- **Do not route look-at through ToDo** (the Finding 8 wording); it would diverge from the
  decompile and add latency to a read-only query.

## 8. Tests

- Builder queue shapes: `Use` single → `[Wait{100}, Use]`; `Use` two-object → `[Wait{100}, Use]`
  with `obj2=Some`; `Move`/`Trade` → single entry, no wait; `Turn` → `[Wait{100}, Turn]`.
- Builder failure: unreachable/absent object → `Err(NOTACCESSIBLE)` at enqueue (handler
  `SendResult`).
- Gate: two-object use twice within 1000 ms → second defers to `earliest_multiuse_server_ms`;
  single-object use back-to-back → no extra delay beyond `Wait{100}`.
- Execute: `Use` runs at beat drain (not at packet receipt) — assert `server_ms` advanced a beat
  between enqueue and effect; success sets multiuse exhaustion.
- Catch path: executor `EXHAUSTED` → `ToDoWait(1000)` + `ToDoStart` + `SendResult`; other
  `RESULT` → `ToDoYield` + `SendResult` (+ snapback except the three exempt results).
- Walk-to-reach: use on a far object → `Go` steps then `Use` fires when adjacent (reuses the
  existing walk-to-use assertions, restated on the unified path).
- Regression: `LookAt` still answers immediately (no ToDo entry created).

## 9. Verification

```bash
rtk cargo check
rtk cargo clippy --all-targets
rtk cargo test -p tfs-rust-core
```
Watch suites: `creature_todo`, `idle_stimulus` (`execute_creature_todo_action`), `container_ui`,
`walk_action`, `game_loop` (`game_packet_requires_timed_action` tests — update as opcodes move
onto the ToDo gate).

## 10. C++ reference index (cite in module headers)

| Piece | File:function |
|-------|---------------|
| ToDo entry union | `cr.hh:461-513` `TToDoEntry` |
| Use handler | `receiving.cc:384` `CUseObject`, `:430` `CUseTwoObjects` |
| Move handler | `receiving.cc:233` `CMoveObject` |
| Trade handler | `receiving.cc:290` `CTradeObject` |
| Turn handler | `receiving.cc:~560` `CTurn` |
| Talk handler | `receiving.cc:750` `CTalk` |
| Look (immediate) | `receiving.cc:717` `CLookAtPoint` → `Look()` |
| Builders | `cract.cc:1123` `ToDoMove`, `:1202` `ToDoTrade`, `:1258` `ToDoUse`, `:1326` `ToDoTurn`, `:1367` `ToDoTalk` |
| Start | `cract.cc:955-1024` `ToDoStart` |
| Delay gate | `cract.cc:901-960` `CalculateDelay` |
| Execute drain | `cract.cc:783-898` `Execute` |
| Use executor + reach + multiuse | `cract.cc:~600-765` `Use` (`EarliestMultiuseTime` `:765`) |
