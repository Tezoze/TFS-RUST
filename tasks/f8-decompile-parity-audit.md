# F8 Player-Action Builders — Decompile Parity Audit

**Scope.** Audit of the three F8 ToDo builders against the CipSoft 7.72 decompile
(`reference/cipsoft-772/tibia-game-master/src/`):

| Rust builder | Rust packet(s) | C++ builder | C++ handler |
|---|---|---|---|
| `enqueue_player_use` | `UseItem` / `UseItemEx` | `TCreature::ToDoUse` (`cract.cc:1258-1323`) | `CUseObject` (`receiving.cc:384`), `CUseTwoObjects` (`receiving.cc:430`) |
| `enqueue_player_move` | `Throw` | `TCreature::ToDoMove` (`cract.cc:1123-1172`) | `CMoveObject` (`receiving.cc:233`) |
| `enqueue_player_turn` | `RotateItem` | `TCreature::ToDoTurn` (`cract.cc:1326-1351`) | `CTurnObject` (`receiving.cc:549`) |

Files audited: `crates/tfs-rust-core/src/creature_todo.rs` (builders),
`idle_stimulus.rs` (execute arms), `game_world_player_rotate.rs` (Turn executor),
`game_world_player_throw.rs` (Move executor), `container_ui.rs` (Use executor),
`game_loop.rs` (handler routing).

**Porting model reminder.** We match *observable outcome*, not C++ structure. Several
deviations below are structural (walk-to-reach moved from builder to execute arm) and are
fine **if** the outcome matches. The findings flag where the outcome actually differs.

---

## 0. Verdict summary

| # | Finding | Function(s) | Severity | Outcome differs? | Status |
|---|---------|-------------|----------|------------------|--------|
| D1 | `Move` never enqueues the builder's `ToDoWait(Delay)` (100 ms floor) | `enqueue_player_move` | **High** | Yes — move executes ~100 ms too early | **Fixed** — builder now prepends `Wait{100}` (`cract.cc:1155,1165`); tests + `game_loop.rs` comment updated |
| D2 | No `UPSTAIRS`/`DOWNSTAIRS` throw for cross-floor map objects | all three | **High** | Yes — wrong `ReturnValue` + no early reject | Open |
| D3 | `Turn` has no walk-to-reach; fails instead of walking | `enqueue_player_turn` + Turn execute arm | **High** | Yes — distant rotate fails | **Fixed** — `Turn` execute arm (`idle_stimulus.rs`) now mirrors the S5 `Go`-prepend: not-adjacent map tile → `setup_player_walk_to_target` + `[Go, Turn]`; no-path → `apply_todo_result_catch` (`cract.cc:1340-1341`); tests + builder doc comment updated |
| D4 | Single-object `Use` C++ path enqueues **two** `Wait{100}` (handler + builder); Rust enqueues one | `enqueue_player_use` | Medium | Yes (verify) — ~100 ms vs ~200 ms floor | Open |
| D5 | Walk-to-reach moved from builder to execute arm changes the `NOWAY` snapback/clear timing | `Use`/`Move` | Medium | Possibly — snapback path differs | Open |
| D6 | Range gate uses `look_distance_tfs` (+15 for Δz) not `ObjectInRange(1)` (strict same-z) | all three | Medium | Only on Δz (folds into D2) | Open |
| D7 | `Wait` drain ignores `EarliestWalkTime`; relative-delay re-anchoring | all (via `Wait`) | Low | Minor timing | Open |
| D8 | Handler flag pre-validation absent (`MULTIUSE`, `isMapContainer`, `CUMULATIVE&&Count==0`, `Dummy` bound) | all three | Low | Edge/robustness | Open |
| D9 | `Move` creature-container branch (`Delay=1000`, BANK dest) not ported | `enqueue_player_move` | Low | Out of scope (creature push) | Open (blocked on creature push) |

---

## 1. `enqueue_player_move` vs `ToDoMove` — **most divergent**

### C++ reference (`cract.cc:1123-1172`, coordinate overload)

```cpp
void TCreature::ToDoMove(int ObjX, ..., int DestX, int DestY, int DestZ, uint8 Count){
    Object Obj = GetObject(this->ID, ObjX, ObjY, ObjZ, RNum, Type);
    if(!Obj.exists()) throw NOTACCESSIBLE;

    if(ObjX != 0xFFFF){                       // map tile
        if(this->posz > ObjZ) throw UPSTAIRS;
        else if(this->posz < ObjZ) throw DOWNSTAIRS;
        if(!ObjectInRange(this->ID, Obj, 1))
            this->ToDoGo(ObjX, ObjY, ObjZ, false, INT_MAX);   // walk-to-reach
    }

    int Delay = 100;
    if(Obj.getObjectType().isCreatureContainer()){            // pushing a body
        Object DestBank = GetFirstObject(DestX, DestY, DestZ);
        if(DestBank == NONE || !DestBank.getObjectType().getFlag(BANK)) throw NOTACCESSIBLE;
        ...
        Delay = 1000;
        if(this->EarliestWalkTime > ServerMilliseconds)
            Delay += (int)(this->EarliestWalkTime - ServerMilliseconds);
    }

    this->ToDoWait(Delay);                      // <-- builder ALWAYS adds a Wait
    TToDoEntry TD = {}; TD.Code = TDMove; ...   TD.Move.Count = Count;
    this->ToDoAdd(TD);
}
```

The `CMoveObject` handler (`receiving.cc:233`) itself calls only `ToDoMove(...)` +
`ToDoStart()` — no leading `ToDoWait`. **But the builder appends one.** Resulting queue for
an adjacent map item: `[Wait{100}, Move]`.

### Rust (`creature_todo.rs` `enqueue_player_move`)

```rust
pub(crate) fn enqueue_player_move(&mut self, cid, obj, dest, count) -> Result<(), ReturnValue> {
    self.validate_move_object_ref(cid, obj)?;                  // GetObject + sprite check
    // NO Wait prepended
    k.base_mut().todo.queue.push_back(CreatureAction::Move { obj, dest, count });
    Ok(())
}
```

Doc comment: *"enqueues `Move` with **no** `Wait` prefix (the decompile's `CMoveObject`
handler calls `ToDoMove` + `ToDoStart` directly — no `ToDoWait`)."*

### D1 — missing 100 ms floor (High) — **Fixed**

The comment (and F8 §0.1 **F5**) conflated the *handler* with the *builder*. The handler
has no `ToDoWait`, but `ToDoMove` **itself** calls `this->ToDoWait(Delay)` with `Delay = 100`.
The Rust builder dropped it entirely, so a throw/move executed on the next beat (delay 0 →
clamped to 1 ms in `todo_start_from_action`) instead of ~100 ms out. **Observable: item
moves faster than the reference.** This also removed the only pacing between rapid move
packets.

**Fix.** `enqueue_player_move` now calls `enqueue_creature_wait(cid, 100)` before pushing
`Move`, matching `cract.cc:1155,1165`. The resulting queue `[Wait{100}, Move]` mirrors the
existing `Use`/`Turn` builder shape, and the `game_loop.rs` `CMoveObject` arm still calls
`todo_start_from_action(cid, 1)` after enqueue (consistent with `Use`/`Turn`). The
creature-container `Delay = 1000` branch (D9) is deliberately not ported yet — the
non-creature-container `Delay = 100` is the only path that fires today. Tests updated:
`enqueue_player_move_prepends_wait_then_move` (renamed from `…_no_wait`) and
`s5_move_not_adjacent_prepends_go_and_re_enqueues_move` (now drains the `Wait{100}` before
the `Move` arm runs the `Go`-prepend).

### D2 — missing `UPSTAIRS`/`DOWNSTAIRS` (High, shared with Use/Turn)

For map-tile sources the builder must throw `UPSTAIRS` (player above object) or `DOWNSTAIRS`
(player below) *before* any walk attempt (`cract.cc:1131-1135`). Rust has no such check in the
builder. The Move *executor* (`player_move_item`, `game_world_player_throw.rs`) does have
`FirstGoUpStairs`/`FirstGoDownStairs` z-checks, so Move is partially covered on the executor
side — but the *source-object* z-check at enqueue is still absent, and the execute-arm
`needs_walk` test (`dx > 1 || dy > 1`) ignores z, so a cross-floor source is misrouted into a
walk attempt first.

### D9 — creature-container branch not ported (Low)

`isCreatureContainer()` (pushing a player/monster body) uses `Delay = 1000` + a `BANK`
destination check + `Combat.DelayAttack(2000)` (`cract.cc:475-480`). Creature push isn't
ported; note it so the `Delay=1000` path is added when it is.

---

## 2. `enqueue_player_turn` vs `ToDoTurn`

### C++ reference (`cract.cc:1326-1351`)

```cpp
void TCreature::ToDoTurn(int ObjX, int ObjY, int ObjZ, ObjectType Type, uint8 RNum){
    Object Obj = GetObject(this->ID, ObjX, ObjY, ObjZ, RNum, Type);
    if(!Obj.exists()) throw NOTACCESSIBLE;

    if(ObjX != 0xFFFF){
        if(this->posz > ObjZ) throw UPSTAIRS;
        else if(this->posz < ObjZ) throw DOWNSTAIRS;
        if(!ObjectInRange(this->ID, Obj, 1))
            this->ToDoGo(ObjX, ObjY, ObjZ, false, INT_MAX);   // walk-to-reach
    }

    this->ToDoWait(100);
    TToDoEntry TD = {}; TD.Code = TDTurn; TD.Turn.Obj = Obj.ObjectID;
    this->ToDoAdd(TD);
}
```

### Rust (`enqueue_player_turn`)

```rust
self.validate_action_object_ref(cid, obj)?;
self.enqueue_creature_wait(cid, 100);          // Wait{100} ✓
k.base_mut().todo.queue.push_back(CreatureAction::Turn { obj });
```

`Wait{100}` is correct. But:

### D3 — Turn never walks to reach the object (High)

Two-part gap:
1. The builder omits the `ObjectInRange(1)` → `ToDoGo(...)` prepend.
2. Unlike the `Use`/`Move` execute arms (which got the S5 `Go`-prepend), the **`Turn`
   execute arm has no walk-to-reach at all** (`idle_stimulus.rs`): it calls
   `player_rotate_item` directly, which returns `NotPossible` when
   `look_distance_tfs(player_pos, obj.pos) > 1` (`game_world_player_rotate.rs`).

Result: rotating a rotatable object more than 1 tile away **fails with a cancel** in Rust,
whereas the reference walks the player adjacent and then rotates. This is the clearest
behavioral regression of the three.

### D2 — same missing `UPSTAIRS`/`DOWNSTAIRS` as above.

### Executor note (informational, not a divergence)

`player_rotate_item` implements `Turn`→`::Turn`→`Change(Obj, RotateTarget, 0)`
(`operate.cc:2562-2583`, `:1534-1638`) as a direct `item.item_type = rotate_to` + `0x6B`
broadcast. For simple map rotatables (torches, ropes) that is outcome-equivalent to `Change`.
Two minor points:
- C++ rejects with `NOTTURNABLE` when `!getFlag(ROTATE)`; Rust maps that to `NotPossible`.
  Acceptable (no exact Rust variant), but the *result byte* sent to the client differs from
  the reference's `NOTTURNABLE` (57).
- C++ self-destruct guard is `RotateTarget == ObjType` (rotateto points to itself), which it
  logs then still runs; Rust guards `rotate_to == 0`. Different predicate — confirm which
  rotatable items (if any) have `rotateto == 0` vs `rotateto == self` in the 772 dataset.

---

## 3. `enqueue_player_use` vs `ToDoUse`

### C++ reference (`cract.cc:1258-1296`, coordinate overload — the one the handlers call)

```cpp
void TCreature::ToDoUse(uint8 Count, int ObjX1,..., uint8 Dummy, int ObjX2,...){
    Object Obj1 = GetObject(...); if(!Obj1.exists()) throw NOTACCESSIBLE;
    Object Obj2 = NONE;
    if(Count >= 2){ Obj2 = GetObject(...); if(!Obj2.exists()) throw NOTACCESSIBLE; }

    if(ObjX1 != 0xFFFF){
        if(this->posz > ObjZ1) throw UPSTAIRS;
        else if(this->posz < ObjZ1) throw DOWNSTAIRS;
        if(!ObjectInRange(this->ID, Obj1, 1))
            this->ToDoGo(ObjX1, ObjY1, ObjZ1, false, INT_MAX);
    }

    this->ToDoWait(100);                        // builder Wait
    TToDoEntry TD = {}; TD.Code = TDUse; TD.Use.Obj1=...; TD.Use.Obj2=...; TD.Use.Dummy=Dummy;
    this->ToDoAdd(TD);
}
```

Handlers (`receiving.cc:384` / `:430`):

```cpp
Player->ToDoWait(100);        // handler Wait  (BEFORE the builder)
Player->ToDoUse(1|2, ...);    // builder adds ANOTHER Wait(100)
Player->ToDoStart();
```

### Rust (`enqueue_player_use`)

```rust
self.validate_action_object_ref(cid, obj1)?;
if let Some(o2) = obj2 { self.validate_action_object_ref(cid, o2)?; }
self.enqueue_creature_wait(cid, 100);          // ONE Wait{100}
k.base_mut().todo.queue.push_back(CreatureAction::Use { obj1, obj2, open_index });
```

### D4 — single `Wait{100}` vs double (Medium — confirm)

The C++ single-/two-object use path enqueues **two** `ToDoWait(100)` (handler + builder),
i.e. queue `[Wait{100}, (Go...), Wait{100}, Use]` — an effective ~200 ms floor before the use
fires on an adjacent object. Rust enqueues exactly one. If the double-wait is intended 772
behavior (not a decompiler artifact), Rust uses fire ~100 ms too early. **Action:** confirm
against live-client timing / `EarliestMultiuseTime` interplay before changing — but the
decompile as written is authoritative and says two.

### D2 / D5 — z-check + walk-to-reach.

`Use` got the S5 `Go`-prepend in the execute arm, so walk-to-reach *outcome* is largely
covered. Remaining gaps: (a) no `UPSTAIRS`/`DOWNSTAIRS` (D2) — a cross-floor use is routed
into pathfinding that fails with `ThereIsNoWay` rather than the reference's up/down-stairs
result; (b) the builder-vs-execute-arm relocation (D5, below).

### D8 — handler flag pre-validation absent (Low)

`CUseObject` rejects `Type.getFlag(MULTIUSE)` and requires `Dummy < NARRAY(OpenContainer)`;
`CUseTwoObjects` requires `Type1.getFlag(MULTIUSE)`; both reject `isMapContainer()`. `CMoveObject`
rejects `isMapContainer() || (CUMULATIVE && Count==0)`. These are silent early returns in
`receiving.cc` that keep malformed/ambiguous commands out of the ToDo queue. Rust's
`game_loop.rs` arms don't replicate the MULTIUSE gating (which is what decides single- vs
two-object dispatch in C++). Low impact for well-behaved clients; matters for robustness and
for picking the correct use path.

---

## 4. Cross-cutting findings

### D5 — walk-to-reach relocated from builder to execute arm (Medium)

C++ resolves reach **at enqueue** inside the builder: `ToDoGo(...)` is prepended, and if
`ToDoGo`'s pathfinder fails it calls `ToDoClear()` + `SendSnapback` and throws `NOWAY`
(`cract.cc:1093-1099`) — i.e. the *builder* both plans the walk and handles no-path with an
immediate snapback. Rust (S5) instead defers this to the execute arm
(`setup_player_walk_to_target` + `Go`-prepend; on `Err` → `apply_todo_result_catch`). The
end state is similar, but:
- The C++ no-path snapback happens at **packet-receipt time**; Rust's happens a beat later at
  **execute time**. Observable ordering of the cancel/snapback differs.
- C++ `ToDoGo` collapses a 1-step path to a single `TDGo` and only runs the pathfinder for
  ≥2 steps (`cract.cc:1082-1108`); Rust always calls `setup_player_walk_to_target`. Confirm
  the single-step fast path is preserved (perf + identical step emission).

This is an accepted structural port **provided** the snapback timing shift is acceptable
under the beat model. Flag for parity testing rather than a hard bug.

### D6 — range predicate mismatch (Medium, folds into D2)

C++ uses `ObjectInRange(ID, Obj, 1)` = `posz == ObjZ && |dx| <= 1 && |dy| <= 1`
(`info.cc:233-257`). Rust uses `look_distance_tfs(pp, pos) > 1` = `max(|dx|,|dy|) > 1`, plus
`+15` when `z` differs. For **same z** these are equivalent (`max>1 ⟺ !(dx<=1&&dy<=1)`). For
**different z**, `ObjectInRange` is false (→ C++ would already have thrown UP/DOWNSTAIRS
before reaching it), while Rust's `+15` silently turns it into a "walk" — the D2 symptom.
`look_distance_tfs` is a *look* helper (`game.cpp` playerLookAt), not the *reach* predicate;
using it for reach is the root of D6/D2.

### D7 — `Wait` drain semantics (Low)

C++ `TDWait` stores `Wait.Time = ServerMs + Delay` (absolute, captured at enqueue) and
`CalculateDelay(TDWait)` returns `max(Wait.Time, EarliestWalkTime) - ServerMs`
(`cract.cc:906-918`). Rust `CreatureAction::Wait { delay_ms }` stores a **relative** delay and
on drain calls `todo_start_from_action(cid, delay_ms)` — re-anchored to drain time and
**ignoring `EarliestWalkTime`**. Two consequences:
1. A wait enqueued at T but drained at T+beat waits a *fresh* 100 ms from the drain instant,
   not the remaining `Wait.Time - now`. Slightly longer than reference.
2. The `max(..., EarliestWalkTime)` coupling (a use/turn right after a walk step is delayed
   until the walk cooldown clears) is lost. Minor unless clients chain walk→use tightly.

---

## 5. Remediation checklist (make Rust match)

Ordered by severity. Each item cites the C++ anchor to preserve in a module-header comment.

- [x] **D1 — add the `Move` builder `Wait`.** In `enqueue_player_move`, prepend
  `enqueue_creature_wait(cid, 100)` before pushing `Move` (`cract.cc:1160`,
  `ToDoWait(Delay)` with `Delay=100`). Update the (incorrect) doc comment and F8 §0.1 F5 note.
- [x] **D3 — give `Turn` walk-to-reach.** Mirror the `Use`/`Move` S5 `Go`-prepend in the
  `CreatureAction::Turn` execute arm (`idle_stimulus.rs`): if `obj.pos` is a map tile and
  `dx>1 || dy>1`, `setup_player_walk_to_target` + push `[Go, Turn]` instead of calling
  `player_rotate_item` immediately (`cract.cc:1338-1339` `ToDoGo`).
- [ ] **D2/D6 — add z-checks + fix the reach predicate.** In all three builders, for
  `obj.pos.x != 0xFFFF`: if `player.z > obj.z` → `Err(FirstGoUpStairs)`, if `<` →
  `Err(FirstGoDownStairs)` (`cract.cc:1131-1135/1272-1276/1332-1336`). Replace the
  `look_distance_tfs`-based reach test in the execute arms with a same-z Chebyshev `ObjectInRange(1)`
  equivalent so Δz no longer masquerades as "needs walk".
- [ ] **D4 — resolve the double-`Wait`.** Confirm whether 772 use really floors at ~200 ms
  (handler `ToDoWait(100)` + builder `ToDoWait(100)`). If yes, enqueue two `Wait{100}` for
  `Use` (and keep one for `Turn`, whose handler has no extra wait — `CTurnObject` calls only
  the builder). Document the decision either way.
- [ ] **D5 — parity-test the relocated walk-to-reach.** Add a test that a no-path `Use`/`Move`
  produces the cancel/snapback with the expected beat timing, and that a 1-step reach emits a
  single `Go` (no pathfinder) matching `cract.cc:1082-1090`.
- [ ] **D7 — fold `EarliestWalkTime` into `Wait` drain** (optional, low): compute the `Wait`
  delay as `max(stored_wait_deadline, earliest_walk_ms) - server_ms` to match
  `CalculateDelay(TDWait)`.
- [ ] **D8 — add handler flag pre-checks** (low/robustness): in the `game_loop.rs` arms (or
  the builders), reject `isMapContainer`, gate single- vs two-object use on `MULTIUSE`, reject
  `CUMULATIVE && count==0` for `Move`, and bound `open_index` (`receiving.cc:384/430/233`).
- [ ] **D9 — creature-container `Move`** (deferred): add the `Delay=1000` + `BANK` dest branch
  when creature push is ported (`cract.cc:475-480`, builder `:1145-1160`).

---

## 6. What is already correct

- `enqueue_player_turn` prepends `Wait{100}` — matches `ToDoTurn`'s `ToDoWait(100)`.
- All three resolve the object **at enqueue** and return `Err(NotPossible)` on failure —
  matches `GetObject` → `throw NOTACCESSIBLE`. Location+sprite re-validation (not a cached
  `ItemId`) correctly mirrors `Obj.exists()` and avoids SlotMap generation reuse (F8 §7).
- Two-object `Use` multiuse gate (`earliest_multiuse_server_ms`) is applied in the execute arm
  — matches `CalculateDelay(TDUse)` gating on `Obj2 != 0` (`cract.cc:925-932`) and
  `EarliestMultiuseTime = ServerMs + 1000` (`cract.cc:765`).
- The `RESULT` catch (`apply_todo_result_catch`) mirrors `Execute`'s `catch(RESULT)`
  (`cract.cc:870-889`): `EXHAUSTED → Wait{1000}`, else `ToDoYield`, player `SendResult` +
  conditional snapback exemptions.
- `player_rotate_item` reproduces `Turn`→`Change` for simple map rotatables (in-place type
  swap + `0x6B`), which is outcome-equivalent for the rotatable object set.
