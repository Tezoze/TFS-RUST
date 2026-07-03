# F8 Sub-Plan — Route Player Actions Through the ToDo `Execute` Engine

**Parent:** `tasks/unified-beat-engine-phases.md` Phase 0 / Finding **F8**.
**Audit source:** `docs/GAME_LOOP_772_AUDIT.md` Finding 8.
**Decompile ref tree:** `reference/cipsoft-772/tibia-game-master/src/` — `receiving.cc`,
`cract.cc`, `cr.hh`. **(772 mechanics = `tibia-game-master/src/`; do not cite `gameserver/src/`
or repo-root `src/` for this ToDo model.)**
**Status:** 🟡 S0–S4 DONE — S5–S8 pending.
**S0 decision (recorded):** Narrow scope. In-scope executors: `Use`/`UseItemEx` (reroute),
`Throw`/`Move` (reroute), `RotateItem`/`CTurnObject` (build + route). **Out of scope, forked to
their own sub-plans:** `UseOnCreature`/`CUseOnCreature` (§0.1 F3 — build-from-scratch), player
`Say`/`ToDoTalk` (§0.1 F4 — build-from-scratch + `CreatureAction::Talk` type change). `Trade`
(§0.1 F1) dropped either way — not ported. §1/§4/§5/§6 updated to match this decision;
§0.1's audit findings (F1–F6) and §10's reference index are left as-is since they document the
audit rationale and C++ references (not active scope).

> **Audit pass (this revision).** Re-verified every claim against the current tree
> (`crates/tfs-rust-core`, `tfs-rust-common`) and the `receiving.cc` reference. Section 0.1 below
> lists corrections — the biggest one is that several "already reactive" pieces this plan assumes
> exist (`Trade`, `Turn`/`CTurnObject`, `UseOnCreature`, `Say`) are **not implemented at all**, not
> merely mis-routed. That changes the actual size of S2/S4/S6 for those four pieces from "move an
> existing executor into a ToDo arm" to "write the executor from scratch, then route it through
> ToDo." Also fixes a packet-name mismatch (`MoveObject`/`TurnObject` aren't real Rust enum variants)
> and a blocking type mismatch in the proposed `Talk` reuse (`enqueue_creature_talk` takes
> `&'static str`, which cannot carry player-typed chat text).

## 0.1 Audit findings

### F1 — `Trade` is not implemented anywhere in Rust (not "reactive," fully missing)
The plan's §1 table and §4 step 7 both frame `Trade`/`CTradeObject` as an existing reactive path to
reroute onto `enqueue_player_trade` + a `player_trade` executor. Checked `game_loop.rs` +
grepped the whole crate: `GamePacket::RequestTrade`, `LookInTrade`, `AcceptTrade`, `CloseTrade` are
all parsed (`game_packet.rs`) but **none of them have a match arm in `handle_game_packet`** — they
fall through to the generic `_ => trace!(...)` catch-all. There is no `player_trade` function, no
trade-state struct, and `player_inventory_query_add.rs::player_trade_item` /
`player_inventory_notifications.rs`'s two `checkTradeState` comments are explicit stubs ("Trade
`checkTradeState` when trade port lands"). **The trading system has not been ported at all** — this
sub-plan is not the place to build it. Recommend dropping `Trade` from this sub-plan's scope
entirely (S1–S8) and tracking it as its own future port; when that port happens, route it through
`enqueue_player_trade` from day one (this plan's target shape is still the right end-state for it),
but building the trade feature is a separate, much larger unit of work than "move an existing
executor into ToDo," which is what this document sizes it as.

### F2 — `Turn`/`CTurnObject` (rotate an *item*, not the player) has no executor either, and the Rust `Turn` name is already taken by something else
Two separate problems here:
1. **Naming collision.** `GamePacket::Turn(Direction)` already exists in Rust and maps to CipSoft's
   `CRotate` (player facing direction — `receiving.cc:213`, dispatched immediately, no ToDo entry,
   already handled by `player_turn_request`). That is a *different* packet from the one this plan
   means by "Turn": `CTurnObject` (`receiving.cc:549`, rotate a rotatable object like a wall
   torch/rope) — `ToDoWait(100)+ToDoTurn+ToDoStart`. The Rust enum variant for `CTurnObject` is
   `GamePacket::RotateItem { pos, sprite_id, stack_pos }`, not `Turn`. §1's table, §4 step 7, and
   §10's reference index all say "`Turn` handler" / "`CTurn`" without disambiguating — rewrite these
   to say `RotateItem`/`CTurnObject` so nobody wires the wrong packet.
2. **No executor exists.** Grepped for any rotate-item implementation — `GamePacket::RotateItem`
   also falls through the `_ => trace!` catch-all in `game_loop.rs`. There's no
   `player_rotate_object`/equivalent anywhere in the crate. Same shape as F1: this is "write the
   feature" work, not "reroute an existing reactive path." Small enough (single-object, no
   reach-walk per the decompile — `CTurnObject` has no adjacency/walk-to logic, just a visibility
   check) that it's reasonable to keep in scope, but §6's checklist should say "implement + route,"
   not just "route."

### F3 — `CUseOnCreature` (use-with-creature) also has no executor
The plan's §1 table lists `CUseOnCreature` alongside `CUseObject`/`CUseTwoObjects` as an existing
`ToDoUse`-routable reactive path. Checked: `GamePacket::UseWithCreature { from_pos, sprite_id,
from_stack_pos, creature_id }` is parsed but has **no match arm** in `handle_game_packet` — falls
through to the catch-all, same as F1/F2. Only `UseItem`/`UseItemEx` (map/inventory targets) are
wired; the creature-target variant of use (rune-on-creature, tool-on-creature) isn't implemented.
§4 step 4's "reuse the existing `player_use_item`/`player_use_item_ex` bodies as the executor" is
accurate for those two, but there is a third `Use` variant (two-object-with-creature-target) this
plan's own reference table (§2.1/§10) calls out that has nothing to reuse. Either scope it out
explicitly (build it later, same as Trade) or budget it as new executor work in S4, not a move.

### F4 — Player-issued `Say` is not reactive today; it isn't routed at all, and `Talk` can't be reused as designed
§1's table and §3 both state `Say` currently executes "reactively" and should be rerouted onto
`ToDoTalk`. Checked `game_loop.rs`: `GamePacket::Say(_)` is **not matched anywhere** — it falls
through to the catch-all trace, identically to Trade/RotateItem/UseWithCreature. So today a player
typing in chat produces no broadcast at all (confirmed no caller of `broadcast_creature_say_viewport`
or `enqueue_creature_talk` exists for a player-originated `Say` packet — the only caller is the
monster idle-chatter path, tested in `idle_stimulus_tests.rs::test_phase1_talk_action_broadcasts`).
Separately, the design in §4 step 1/step 2 implies reusing `CreatureAction::Talk { text }` +
`enqueue_creature_talk` for player speech. **This won't compile as designed**:
`enqueue_creature_talk(cid, text: &'static str)` and `CreatureAction::Talk { text: &'static str }`
are deliberately `&'static str` (per the doc comment, to avoid allocation for canned monster lines
like "Hicks!"). Player chat text is an arbitrary runtime `String` (`SayPayload.text: String`) — it
cannot be coerced into `&'static str`. `CreatureAction::Talk` needs either a second variant
(`TalkOwned { text: String }` or similar) or `Talk`'s field needs to become an owned/shared type
(`Box<str>`/`Arc<str>`/`String`) before player speech can be enqueued through it. This is a real
blocker for S1/S2, not just a missing route — flag it before implementation starts so the enum
change is planned rather than discovered mid-PR.

### F5 — `MoveObject` in this plan is `Throw` in Rust, and it already has reach-to-item logic (bigger head start than the plan implies, but wrong name throughout)
§1's table cites `CMoveObject` (`receiving.cc:233`) and §4/§10 refer to a `player_move_object`
executor to add. There is no `GamePacket::MoveObject` — the corresponding Rust packet is
`GamePacket::Throw(ThrowPayload)`, dispatched to `player_move_thing` →
`player_move_item` (`game_world_player_throw.rs`). Unlike Trade/RotateItem/UseWithCreature, **this
one is real and working** — it already has its own walk-to-reach logic
(`throw_dest_reachable_after_walk_to_item` + `try_walk_to_and_action`), mirroring the pattern
`player_use_item` uses. So `Move`/`MoveObject` is in the same boat as `Use`: an existing reactive
executor that genuinely can be moved into a ToDo arm (not built from scratch) — the plan's
size estimate for this one is correct, just the packet name (`MoveObject`) should be `Throw`
throughout (§1 table, §4 step 7, §5 table, §10 reference index) so implementers grep the right enum
variant.

### F6 — Corrected scope summary for §1's table
Given F1–F5, the "route through ToDo" column is right for 2 of 6 rows; the other 4 need the
executor built first (or descoped). Corrected view:

| Client command | Rust packet name | Executor exists today? | This plan's real scope |
|---|---|---|---|
| `CUseObject` | `UseItem` | Yes (`player_use_item`) | Reroute only (as written) |
| `CUseTwoObjects` | `UseItemEx` | Yes (`player_use_item_ex`) | Reroute only (as written) |
| `CUseOnCreature` | `UseWithCreature` | **No** (F3) | Build + route, or descope |
| `CMoveObject` | `Throw` | Yes (`player_move_thing`/`player_move_item`) | Reroute only, fix packet name (F5) |
| `CTradeObject` | `RequestTrade`/`AcceptTrade`/`CloseTrade`/`LookInTrade` | **No** (F1) | Out of scope — separate port |
| `CTurnObject` | `RotateItem` | **No** (F2) | Build + route, fix packet name |
| `CTalk` (local) | `Say` | **No** (F4) | Build + route; also needs a `CreatureAction::Talk` type change |
| `CLookAtPoint` | `LookAt` | Yes (`player_look_at`), stays reactive | No change (correct as written) |

Recommend narrowing this sub-plan's S1–S8 to the two "reroute only" rows (`Use`/`UseTwoObjects`,
`Throw`) plus `RotateItem` (small, self-contained) for a first pass, and explicitly forking
`UseOnCreature`, `Trade`, and `Say`-as-`ToDoTalk` into follow-up work items — each of those three is
"implement a missing feature," not "close Finding 8's routing gap," and bundling them into this
plan's checklist understates the work by treating them as done-except-for-wiring.

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

| Client command | Rust packet | 772 handling | Route through ToDo? | Executor exists today? |
|---|---|---|---|---|
| `CUseObject` (single) `receiving.cc:384` | `UseItem` | `ToDoWait(100)` + `ToDoUse(1,…)` + `ToDoStart` | **Yes** | Yes — `player_use_item` |
| `CUseTwoObjects` `receiving.cc:430` | `UseItemEx` | `ToDoWait(100)` + `ToDoUse(2,…)` + `ToDoStart` | **Yes** | Yes — `player_use_item_ex` |
| `CUseOnCreature` `receiving.cc:480` | `UseWithCreature` | `ToDoWait(100)` + `ToDoUse(2, Obj, CrObj)` + `ToDoStart` | **Out of scope per S0** — fork to own sub-plan | **No** (§0.1 F3) |
| `CMoveObject` `receiving.cc:233` | `Throw` | `ToDoMove(…)` + `ToDoStart` (no wait) | **Yes** | Yes — `player_move_thing`/`player_move_item` (§0.1 F5) |
| `CTradeObject` `receiving.cc:290` | `RequestTrade`/`AcceptTrade`/`CloseTrade`/`LookInTrade` | `ToDoTrade(…)` + `ToDoStart` (no wait) | **Out of scope** — feature doesn't exist (§0.1 F1) | **No** |
| `CTurnObject` (rotate an object) `receiving.cc:549` | `RotateItem` | `ToDoWait(100)` + `ToDoTurn(…)` + `ToDoStart` | **Yes, but build first** | **No** (§0.1 F2) |
| `CTalk` (say/yell/whisper) `receiving.cc:750` | `Say` | `ToDoTalk(…)` + `ToDoStart` (no wait) | **Out of scope per S0** — fork to own sub-plan | **No** — player `Say` isn't wired at all today (§0.1 F4); also needs a `CreatureAction::Talk` type change (owned text, not `&'static str`) |
| `CLookAtPoint` `receiving.cc:717` | `LookAt` | `Look(ID, Obj)` **immediately**; no ToDo entry | **No — stays reactive** | Yes — `player_look_at` (correct as-is) |
| `CInspectTrade` `receiving.cc:337`, `UpdateContainer`/`BrowseField` refresh reads | `LookInTrade`, `UpdateContainer`, `BrowseField` | immediate reads | **No — stays reactive** | `LookInTrade` has no executor either (blocked on Trade, F1); container refresh reads exist and are fine |

**Note on `CRotate` vs `CTurnObject`:** don't confuse these. `CRotate` (`receiving.cc:213`, player
facing direction) is unrelated to this plan and already maps to `GamePacket::Turn(Direction)` →
`player_turn_request`, dispatched immediately with no ToDo entry — leave it alone. This plan's
"Turn" is `CTurnObject` (rotate a rotatable *item*), which is `GamePacket::RotateItem` in Rust, not
`Turn`. See §0.1 F2.

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
  `Talk{text: &'static str}`. **Missing: `Use`, `Move`, `Turn`.** (`Trade` deliberately excluded —
  see §0.1 F1.) **Note:** there is **no** `Rotate{target_id}` variant on `CreatureAction` today —
  the doc comment on the enum explicitly explains `TDRotate` is *not* modeled as a queued action
  (772's idle-combat tail calls `Rotate(Target)` directly, `crnonpl.cc:2872-2873`, so it lands in
  the same beat as the first `TDGo` and stays imperceptible); an earlier revision of this plan's
  variant list included `Rotate{target_id}` in error — corrected here to match the actual enum.
- `execute_creature_todo_action` (`idle_stimulus.rs:2222`) dispatches Go/Wait/Talk/Attack.
- Reactive handlers dispatched inline in `game_loop.rs` `handle_game_packet`:
  `player_use_item` / `player_use_item_ex` (`container_ui.rs:503/567`), `player_look_at`
  (`game_world_inventory.rs:936`), `player_move_thing`/`player_move_item` for `Throw`
  (`game_world_player_throw.rs`), container ops. **Not** dispatched anywhere (fall through to the
  catch-all trace in `handle_game_packet`, confirmed by grep): `Say`, `RequestTrade`/`AcceptTrade`/
  `CloseTrade`/`LookInTrade`, `RotateItem`, `UseWithCreature`. See §0.1 for the F1–F5 breakdown —
  these four/five are missing features, not misrouted reactive paths.
- Mitigations already present (keep as fallback until replaced):
  - `game_packet_requires_timed_action` (`game_loop.rs:125`) + `player_packet_action_ready`
    (`game_world.rs:178`) approximate the gate.
  - Walk-to-use already deferred: `walk_action.rs` + `try_run_player_walk_action_from_todo`; the
    same `try_walk_to_and_action` helper is also already used by `player_move_item`/`Throw`
    (`game_world_player_throw.rs::throw_dest_reachable_after_walk_to_item`) — one reach-to-target
    helper, reused by both Use and Move today.
  - `player_use_item_ex_ready` already reads `EarliestMultiuseTime` (`game_world.rs:231`).

## 4. Target Rust design

Extend the existing ToDo machinery — do **not** invent a parallel player-action queue.

1. **Extend `CreatureAction`** (`creature_todo.rs`) with object-carrying variants. Store
   **resolved-at-enqueue identity** (not raw wire coords) so execute-time re-validation is a
   simple lookup:
   ```rust
   enum CreatureAction {
       Go, Wait { delay_ms }, Attack, Talk { text: &'static str }, // existing
       Use { obj1: ActionObjectRef, obj2: Option<ActionObjectRef>, open_index: u8 },
       Move { obj: ActionObjectRef, dest: MoveDest, count: u16 },
       Turn { obj: ActionObjectRef },
       // `Trade` intentionally omitted — trading isn't ported yet (§0.1 F1); out of scope here.
       // `TalkOwned` omitted per S0 — player `Say`/`ToDoTalk` forked to own sub-plan (§0.1 F4).
   }
   ```
   No `Rotate` variant — `CreatureAction` has never had one and shouldn't (§3 note); don't reuse
   that name for the new `Turn`/`CTurnObject` variant either, to avoid confusion with the unrelated
   `GamePacket::Turn` (player-facing-direction) packet.
   `ActionObjectRef` = the Rust analog of `GetObject` result: enough to re-locate the item at
   execute time (map `Position+stackpos` **or** `Inventory{cid,slot}` **or** `Container{cid,slot}`
   + expected item type for validation). This mirrors the decompile's `Object` handle resolved in
   the `ToDo*` builder.
2. **Add `ToDo*` builders** (`creature_todo.rs`, alongside `enqueue_creature_go/wait/talk`):
   `enqueue_player_use`, `enqueue_player_move`, `enqueue_player_turn`
   (`Trade` excluded — §0.1 F1; `TalkOwned` excluded per S0 — §0.1 F4). Each **resolves the object
   now** (via the existing inventory/tile query helpers) and returns `Err(ReturnValue)` if
   resolution fails — mirroring the builder `throw RESULT` at `receiving.cc` enqueue time.
   Use/Turn builders also `enqueue_creature_wait(100)` first. `enqueue_player_turn` and its
   executor are new code, not a move (§0.1 F2). `UseOnCreature` is out of scope per S0 — do not add
   a use-with-creature builder here (§0.1 F3).
3. **`CalculateDelay` arms** — the delay is computed in `todo_start_from_action` /
   `execute_creature_todo_action` today. Add:
   - `Use` with `obj2.is_some()` → gate on `earliest_multiuse_server_ms` (field already exists,
     `creature/base.rs:123`).
   - `Move`/`Turn` → delay 0 (execute in-drain).
   Keep single-object `Use` ungated (matches `Obj2 == 0`).
4. **`Execute` dispatch arms** in `execute_creature_todo_action` (`idle_stimulus.rs`): route
   `Use → player_use_item(_ex)` (reuse — moved, not rewritten, since both already exist),
   `Move → player_move_thing`/`player_move_item` (reuse — already exists, see §0.1 F5; the plan's
   earlier "`player_move_object`" name doesn't match anything in the tree), `Turn → new executor`
   (build — nothing to reuse, §0.1 F2), **re-validating the object** first (`NOTACCESSIBLE` on
   failure). On the executor returning a `ReturnValue` error, run the C++ catch:
   `EXHAUSTED → ToDoWait(1000)+ToDoStart`; else `ToDoYield`; emit `SendResult` + conditional
   snapback. (`player_execute_attack` already models this catch — mirror it.)
5. **Set `EarliestMultiuseTime`** on successful two-object use:
   `earliest_multiuse_server_ms = server_ms + 1000` (`player_apply_multiuse_exhaust` already
   exists, `game_world.rs:243` — call it from the Use executor instead of the reactive path).
6. **Walk-to-reach** = prepend `Go` + re-enqueue the action, exactly like the `Use` executor.
   Fold the existing `walk_action` deferral into this (`Move`/`Use` when not adjacent enqueue a
   `Go` front + themselves, then `ToDoStart`). Note `player_move_item`'s reach-check
   (`throw_dest_reachable_after_walk_to_item`) already uses this exact pattern via
   `try_walk_to_and_action` — same helper, no new reach logic needed for `Move`.
7. **Reroute `handle_game_packet`** (`game_loop.rs`): `UseItem`/`UseItemEx`/`Throw`/`RotateItem`
   arms call the `enqueue_*` builder + `todo_start_from_action` instead of the executor directly.
   `RotateItem` needs its match arm *added* (it has none today, §0.1 F2) before routing. On builder
   `Err(rv)` → `SendResult(rv)` (matches the handler-level `catch(RESULT r)` in `receiving.cc`).
   **Leave `LookAt`, container refresh reactive; leave `Trade`/`UseWithCreature`/`Say` out of this
   plan's routing entirely (§0.1 F1/F3/F4 — all three forked to own sub-plans per S0).**

## 5. Delay / gate parity table (target)

| CreatureAction | Prepend `Wait{100}` | CalculateDelay gate | Executor | Sets exhaustion | Executor status |
|---|---|---|---|---|---|
| `Use{obj2:None}` | yes | none | `player_use_item` | — | exists (reroute) |
| `Use{obj2:Some}` | yes | `earliest_multiuse_server_ms` | `player_use_item_ex` | `+1000 ms` multiuse | exists (reroute) |
| `Move` | no | none (0) | `player_move_thing`/`player_move_item` | — | exists (reroute; §0.1 F5 — not "`player_move_object`") |
| `Turn` | yes | none (0) | new executor (rotate item) | — | **build from scratch** (§0.1 F2) |

`Trade` removed from this table — out of scope, see §0.1 F1. `TalkOwned` and `UseOnCreature`
removed per S0 — both forked to own sub-plans (§0.1 F3/F4). If either is later pulled back in
scope, add the corresponding row with a **build from scratch** executor.

## 6. Phased steps (checklist)

Each step gates on `rtk cargo check && rtk cargo clippy --all-targets && rtk cargo test`. Keep the
reactive path alive behind the existing gate until its ToDo replacement is verified, then delete.

- [x] **S0 — Scope decision (DONE).** Narrow scope locked: `Use`/`UseItemEx` (reroute), `Throw`
      (reroute), `RotateItem` (build+route). Forked to own sub-plans: `UseOnCreature` (§0.1 F3),
      player `Say`/`ToDoTalk` (§0.1 F4). `Trade` dropped (§0.1 F1). §1/§4/§5/§6 updated to match.
- [x] **S1 — `ActionObjectRef` + enum variants.** DONE. Added `ActionObjectRef` struct
      (`pos`/`stack_pos`/`sprite_id` — resolved-at-enqueue identity, no cached `ItemId`) +
      `Use`/`Move`/`Turn` variants to `CreatureAction` + `has_use`/`has_move`/`has_turn` helpers
      on `CreatureTodo`. Stub match arms in `execute_creature_todo_action` (trace + `Wait` kind)
      keep the enum exhaustive; no executor wired yet. Unit test covers all three `has_*`
      helpers. `cargo check` + `clippy --all-targets` + 524 tests pass.
- [x] **S2 — Builders.** DONE. Added `enqueue_player_use`/`enqueue_player_move`/`enqueue_player_turn`
      + `validate_action_object_ref` (Use/Turn path: `resolve_item_at_position` +
      `find_tile_item_by_client_sprite` fallback) + `validate_move_object_ref` (Move path:
      `internal_get_thing_move`). Each resolves the object at enqueue time and returns
      `Err(NotPossible)` on failure (C++ `NOTACCESSIBLE` → `NotPossible`, matching
      `walk/mod.rs:1506`). Use/Turn prepend `Wait{100}`; Move does not. 7 unit tests cover
      all queue shapes (`[Wait{100}, Use]` single/two-object, `[Move]`, `[Wait{100}, Turn]`)
      + failure cases (absent object → `Err(NotPossible)`, queue unchanged). Also fixed
      `pickup_item_type` to set `moveable_override: Some(true)` (gold IS moveable in Tibia;
      `internal_get_thing_move` requires `moveable()`). `cargo check` + `clippy --all-targets`
      + 531 tests pass.
- [x] **S3 — CalculateDelay.** DONE. Wired the C++ `CalculateDelay(TDUse)` gate
      (`cract.cc:925-932`): two-object `Use` (`obj2.is_some()`) defers when
      `earliest_multiuse_server_ms > server_ms`; single-object `Use`, `Move`, and `Turn`
      are ungated (delay 0 — C++ `default` case, `cract.cc:946-948`). Added
      `multiuse_gate_delay_ms(cid, has_obj2)` shared core + peek-based `todo_use_delay_ms`
      (for S6 handler routing) + `TodoExecuteKind::Deferred` variant (no-op in
      `run_monster_todo_execute` — wakeup already armed). The `Use` execute arm calls
      `multiuse_gate_delay_ms` directly with the popped `obj2.is_some()` (not the
      peek-based helper — the action is already popped at that point). 5 tests: 3 unit
      (`todo_use_delay_ms` two-object within gate → remaining delay, single-object → 0,
      two-object past gate → 0) + 2 integration (two-object Use defers with wakeup at
      `earliest_multiuse_server_ms`; single-object Use does not defer, queue drains).
      `cargo check` + `clippy --all-targets` + 536 tests pass.
- [x] **S4 — Execute dispatch.** ✅ DONE
      - Refactored `player_use_item`/`player_use_item_ex` (`container_ui.rs`) and
        `player_move_thing`/`player_move_item` (`game_world_player_throw.rs`) to return
        `Result<(), ReturnValue>` — `Err(rv)` = hard failure; `Ok(())` = success **or**
        walk-to-reach deferral (transitional — S5 folds into `Go`-prepend).
      - Updated reactive callers (`game_loop.rs`, `walk_action.rs`) to wrap `Err` →
        `send_cancel_message` (preserves existing reactive behavior until S7).
      - **Wrote** new `player_rotate_item` executor (`game_world_player_rotate.rs`) for
        `Turn` — re-validates object, checks `rotatable()` + `rotate_to != 0`, transforms
        `item.item_type = rotate_to`, broadcasts `0x6B` for map tiles. C++ ref:
        `operate.cc:2562-2583` `Turn`, `cract.cc:771-777` `TCreature::Turn`.
      - Added `apply_todo_result_catch` helper (`creature_todo.rs`) — C++ `RESULT` catch
        (`cract.cc:870-889`): `ToDoClear` → `EXHAUSTED`→`ToDoWait(1000)`+`ToDoStart`,
        else →`ToDoYield`(`ToDoWait(0)`+`ToDoStart`); player-only `SendResult` +
        conditional `SendSnapback` (exempt: `NOTINVITED`/`ENTERPROTECTIONZONE`/
        `MOVENOTPOSSIBLE`).
      - Wired `Use`/`Move`/`Turn` execute arms in `execute_creature_todo_action`
        (`idle_stimulus.rs`) — S3 multiuse gate still runs first; on gate pass, dispatch
        to executors via `execute_player_use`/`execute_player_move`/`player_rotate_item`;
        `Err(rv)` → `apply_todo_result_catch`. Multiuse exhaustion set inside
        `player_use_item_ex` on two-object success.
      - 10 new tests: Turn executor (rotatable transforms, non-rotatable→Err,
        out-of-range→Err, absent→Err, rotate_to=0→Err); RESULT catch (EXHAUSTED→
        Wait{1000}, non-exhausted→Wait{0}, snapback-exempt no panic); Use/Move execute
        arms (single-object use dispatches, absent object→Err).
      - `cargo check` + `clippy --all-targets` + 546 tests pass.
- [ ] **S5 — Walk-to-reach.** Route not-adjacent Use/Move through `Go`-prepend + re-enqueue;
      retire the bespoke `walk_action_due` path in favor of the unified Go+action enqueue (verify
      `try_run_player_walk_action_from_todo` tests still describe the same outcome). Note `Move`
      already has its own working reach-check (`throw_dest_reachable_after_walk_to_item`) using the
      same `try_walk_to_and_action` helper — this step folds it in, doesn't build it new.
- [ ] **S6 — Reroute/wire handlers.** Point `UseItem`/`UseItemEx`/`Throw`/`RotateItem` arms at
      builder+`ToDoStart`. `RotateItem` needs its match arm *added* to `handle_game_packet` first
      (it currently falls through to the catch-all `_ => trace!`, §0.1 F2). Keep
      `LookAt`/container-refresh reactive. Leave `Trade`/`UseWithCreature`/`Say` unrouted/unbuilt
      (all three forked per S0). Remove rerouted opcodes from `game_packet_requires_timed_action` /
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
  modes reactively (they don't map to `TDTalk`). Per §0.1 F4, `Say` isn't dispatched *at all* today
  (no handler arm, no enqueue path) — this isn't a re-route, it's standing up player chat for the
  first time, gated on an enum change (`CreatureAction::Talk`'s `&'static str` can't carry player
  text). Confirm this is actually in scope before starting S1 (see S0).
- **`Trade`/`UseOnCreature` scope.** Neither is ported — see §0.1 F1/F3. Do not budget them as
  reroutes; they're new features that happen to want the same target shape this plan builds.
- **1098 reuse.** These variants are era-neutral (they're the ToDo model). When Phase 3/4 moves
  1098 onto ToDo, the 1098 delays come from the profile (`attack_speed_ms`, multiuse gate); the
  action *plumbing* is shared. No `beat_driven_loop` branch in the new arms — read the gate from
  the same seam walk already uses.
- **Do not route look-at through ToDo** (the Finding 8 wording); it would diverge from the
  decompile and add latency to a read-only query.

## 8. Tests

- Builder queue shapes: `Use` single → `[Wait{100}, Use]`; `Use` two-object → `[Wait{100}, Use]`
  with `obj2=Some`; `Move` → single entry, no wait; `Turn` → `[Wait{100}, Turn]`. (No `Trade` case
  — out of scope, §0.1 F1.)
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

| Piece | File:function | Rust packet | Rust executor status |
|-------|---------------|-------------|----------------------|
| ToDo entry union | `cr.hh:461-513` `TToDoEntry` | — | — |
| Use handler | `receiving.cc:384` `CUseObject`, `:430` `CUseTwoObjects` | `UseItem`, `UseItemEx` | exists |
| Use-on-creature handler | `receiving.cc:480` `CUseOnCreature` | `UseWithCreature` | **missing** (§0.1 F3) |
| Move handler | `receiving.cc:233` `CMoveObject` | `Throw` (not `MoveObject`) | exists (§0.1 F5) |
| Trade handler | `receiving.cc:290` `CTradeObject` (+ `:337` `CInspectTrade`) | `RequestTrade`/`AcceptTrade`/`CloseTrade`/`LookInTrade` | **missing, out of scope** (§0.1 F1) |
| Turn (rotate item) handler | `receiving.cc:549` `CTurnObject` | `RotateItem` (not `Turn` — that's `CRotate`) | **missing** (§0.1 F2) |
| Rotate (player facing) handler — unrelated, do not confuse | `receiving.cc:213` `CRotate` | `Turn(Direction)` | exists, already immediate, out of scope for this plan |
| Talk handler | `receiving.cc:750` `CTalk` | `Say` | **missing dispatch arm** (§0.1 F4) |
| Look (immediate) | `receiving.cc:717` `CLookAtPoint` → `Look()` | `LookAt` | exists, stays reactive |
| Builders | `cract.cc:1123` `ToDoMove`, `:1202` `ToDoTrade`, `:1258` `ToDoUse`, `:1326` `ToDoTurn`, `:1367` `ToDoTalk` | — | — |
| Start | `cract.cc:955-1024` `ToDoStart` | — | — |
| Delay gate | `cract.cc:901-960` `CalculateDelay` | — | — |
| Execute drain | `cract.cc:783-898` `Execute` | — | — |
| Use executor + reach + multiuse | `cract.cc:600-765` `Use` (`EarliestMultiuseTime` `:765`) | — | — |
