# 772 NPC Audit — Rust vs Reference Decompile

**Date:** 2026-07-27
**Scope:** NPC dialogue runtime (`TalkStimulus` / `IdleStimulus` / `CreatureMoveStimulus`,
`TBehaviourDatabase::react`, `GiveTo` / `GetFrom` / `GiveMoney` / `GetMoney`, ToDo reply
timing, behaviour-file import) — 772 era only.
**Rust side:**
- `crates/tfs-rust-core/src/npc/` — `focus.rs`, `react.rs`, `match_rule.rs`, `expr.rs`,
  `words.rs`, `host.rs`, `actions.rs`, `stimulus.rs`
- `crates/tfs-rust-core/src/creature_todo.rs`, `idle_stimulus.rs` (ToDo engine)
- `crates/tfs-rust-core/src/player/inventory/money.rs`
- `crates/tfs-rust-content/src/npc_import/` (`lower.rs`, `emit.rs`), `src/npcs/dialogue.rs`
- `crates/tfs-rust-lua/src/npc_dialogue.rs`
- `data/npc/scripts/*.lua` (generated data pack)

**Reference:** `reference/cipsoft-772/tibia-game-master/src/` — `crnonpl.cc`, `cract.cc`,
`operate.cc`, `info.cc`, `strings.cc`, `time.cc`; behaviour sources in
`reference/cipsoft-772/runtime/npc/*.npc`.

Grading: **BUG** (implemented but diverges), **GAP** (reference behavior absent),
**SUSPECT** (probable divergence, needs a targeted check first). `[verified]` = confirmed by
direct side-by-side source comparison during this audit.

---

## Summary table

| # | Sev | Kind | Area | Finding |
|---|-----|------|------|---------|
| 1 | **CRITICAL** | BUG | ToDo engine | `ToDoAdd` never clears an in-flight batch → replies queue behind stale `Wait` deadlines (**user report #1: "weird delay"**) `[verified]` ✅ **Fixed** |
| 2 | **CRITICAL** | BUG | Import lowering | `Create(x)` / `Delete(x)` lower `count` to `Lit(1)`; 772 uses `Npc->Amount` (**user report #2: pay 5, get 1**) `[verified]` ✅ **Fixed** |
| 3 | **CRITICAL** | BUG | Item host | `create_item` does not loop non-cumulative types — one item with `count = N` instead of N items `[verified]` ✅ **Fixed** |
| 4 | HIGH | BUG | Data pack | 623 generated `count = 1` sites across 195 NPC scripts must be regenerated after #2 `[verified]` ✅ **Fixed** |
| 5 | HIGH | GAP | Item host | `Npc->Data` (fluid/key subtype) ignored by `Create` / `Delete` / `Count(...)` `[verified]` — *Phase 2* |
| 6 | HIGH | BUG | Money | Vial-deposit style rules pay `Amount * price` but delete 1 item → gold duplication `[verified]` ✅ **Fixed** (consequence of #2/#3) |
| 7 | MED | BUG | Reply format | `%T` renders PM hours as "am" and uses wall-clock instead of 772 game time `[verified]` — *Phase 2* |
| 8 | MED | GAP | ToDo clear | `player_todo_clear` does not apply pending `TDChangeState` for NPCs → NPC can stick in `Leaving` `[verified]` ✅ **Fixed** |
| 9 | MED | BUG | Stimuli | `CreatureMoveStimulus` VANISH path sets Idle without `ToDoYield` (`ChangeState(IDLE, true)`) `[verified]` — *Phase 2* |
| 10 | MED | BUG | Matching | `%1`/`%2` captures reset per rule; C++ shares one `Parameters[2]` across the whole match loop `[verified]` |
| 11 | LOW | BUG | Actions | `Idle` action sets `StartToDo` even under `ADDRESSQUEUE`; C++ skips it (and logs an error) `[verified]` |
| 12 | LOW | BUG | Reply format | Unknown `%X` escapes and lowercase `%n/%a/%p/%t` diverge from C++ `[verified]` |
| 13 | LOW | GAP | Reply format | No 256-byte reply cap; C++ drops the reply and logs when `FormatNpcResponse` overflows `[verified]` |
| 14 | LOW | SUSPECT | Stimuli | Sleeping-wake is skipped when `focus.is_some()`; C++ wakes regardless of `Interlocutor` |
| 15 | LOW | GAP | Item host | `GetFrom` failure throws `ERROR` in C++ (aborts the reaction); Rust logs and continues |

---

## 1. CRITICAL — ToDo batch is never cleared on re-entry ("weird delay")

**User report:** *"If you talk too quickly to the NPC it will proceed with its functions but
send its message at a weird delay."*

### Reference behavior

`TCreature::ToDoWait` stores an **absolute** deadline, and every enqueue goes through
`ToDoAdd`, which **wipes the whole in-flight list** when a batch is already running:

```c
// cract.cc:991-1000
void TCreature::ToDoAdd(TToDoEntry TD){
    if(this->LockToDo){
        if(this->ToDoClear() && this->Type == PLAYER){
            SendSnapback(this->Connection);
        }
    }
    *this->ToDoList.at(this->NrToDo) = TD;
    this->NrToDo += 1;
}

// cract.cc:1033-1041
void TCreature::ToDoWait(int Delay){
    TD.Wait.Time = ServerMilliseconds + Delay;   // ABSOLUTE
    this->ToDoAdd(TD);
}
```

`TBehaviourDatabase::react` (`crnonpl.cc:1085-1291`) then emits, per reply:
`ToDoWait(TalkDelay)` + `ToDoTalk(...)` with `TalkDelay` accumulating
`3100 + (strlen/2)*100`, and finishes with a trailing `ToDoWait(TalkDelay)` + `ToDoStart()`.
Because `Wait.Time` is absolute, the accumulation is a schedule, not a sum of sleeps.

Net effect in C++: a **new reaction always discards the previous reaction's pending speech**
and schedules from `ServerMilliseconds`. Talking fast = the NPC drops the old line and
answers the new one ~1 s later.

### Rust behavior

`enqueue_creature_wait` / `enqueue_creature_talk` / `enqueue_creature_change_npc_state`
`push_back` unconditionally — no `locked` check, no clear:

- <ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/creature_todo.rs" lines="252-283" />
- <ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/creature_todo.rs" lines="303-321" />

`ToDoClear` is only issued on `ADDRESS` (<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/npc/focus.rs" lines="527-531" />)
and `ADDRESSQUEUE` (<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/npc/focus.rs" lines="160-161" />).
The `DEFAULT` (continuing conversation) and `BUSY` paths never clear.

### Reproduction trace

`server_ms = T`, greeting reply is ~37 bytes → `TalkDelay` ends at ~5900.

| | C++ | Rust |
|---|---|---|
| T: "hi" (ADDRESS) | `ToDoClear`; `[Wait(T+1000), Talk, Wait(T+5900)]` | same |
| T+1000 | Talk fires | Talk fires |
| T+1500: "blank rune" (DEFAULT) | `ToDoAdd` sees `LockToDo` → **clear** → `[Wait(T+2500), Talk, Wait(...)]` | append → `[Wait(T+5900), Wait(T+2500), Talk, Wait(...)]` |
| reply lands | **T+2500** | **T+5900** (drains `Wait(T+5900)` first, then the already-expired `Wait(T+2500)` fires instantly) |

That 3.4 s stall — and the "functions already ran, message comes late" shape, because the
mutating actions in `apply_dialogue_plan` execute *immediately* while only the speech is
queued — is exactly the reported symptom.

### Fix

Add a `ToDoAdd` parity helper in `creature_todo.rs` and route every `enqueue_creature_*`
builder through it:

```rust
/// C++ `TCreature::ToDoAdd` preamble — `cract.cc:991-1000`.
/// Clears an in-flight batch before appending; players get a snapback when a `Go` was pending.
fn creature_todo_add(&mut self, cid: CreatureId, action: CreatureAction) -> bool {
    let locked = self.creatures.get(cid).is_some_and(|k| k.base().todo.locked);
    if locked {
        let had_go = self.player_todo_clear(cid);          // see finding #8 for the ChangeState half
        if had_go {
            if let Some(conn) = self.conn_for_creature(cid) {
                self.enqueue_encoded(conn, /* cancel walk */);
            }
        }
    }
    // push_back(action)
}
```

**Rollout (70 call sites — do not flip all at once):**

1. **Phase A (fixes the report):** route only the NPC dialogue enqueues
   (`npc_schedule_todo_from_plan`, `npc_idle_stimulus` keepalive, `npc_idle_roam_or_sleep`)
   through `creature_todo_add`. NPC ToDo lists only ever contain
   `Wait`/`Talk`/`ChangeNpcState`/`Go`, so blast radius is contained.
2. **Phase B:** audit monster (`idle_stimulus.rs`) and player (`game_loop.rs`) call sites.
   Player packet handlers already emulate the preamble explicitly with
   `player_todo_clear_with_snapback` — those become redundant and should be deleted once
   the helper is universal, not left double-clearing.

**Guard against regression:** `execute_creature_todo_action` re-arms itself with
`push_front` + `todo_start_from_action` (deferred attack). Those must keep using raw
`push_front`, **not** `creature_todo_add`, or the batch self-destructs mid-drain.

---

## 2. CRITICAL — `Create(x)` / `Delete(x)` count comes from `Npc->Amount`, not `1`

**User report:** *"Buying multiple items, ie buy 5 blank runes. Takes the gold for 5 but only
gives one."*

### Reference behavior

`Create` and `Delete` are `BEHAVIOUR_ACTION_FUNCTION1` — the parser accepts **exactly one
argument** (`crnonpl.cc:399-404`, `:443-452`), and the count is always the session variable:

```c
// crnonpl.cc:1177-1178
case 6: Npc->GiveTo(Param, Npc->Amount); break;   // Create(x)
case 7: Npc->GetFrom(Param, Npc->Amount); break;  // Delete(x)
```

Xodet's rune shop relies on this (`reference/cipsoft-772/runtime/npc/xodet.npc:40,46`):

```
%1,1<%1,"blank","rune" -> Type=3147, Amount=%1, Price=10*%1, "... %A blank runes for %P gold?", Topic=1
Topic=1,"yes",CountMoney>=Price -> "Here you are.", DeleteMoney, Create(Type)
```

`Amount = 5`, `Price = 50`. `DeleteMoney` takes `Price`; `Create(Type)` gives `Amount`.

### Rust behavior

<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-content/src/npc_import/lower.rs" lines="294-315" />

```rust
count: count.map(...).transpose()?.unwrap_or(DialogueExpr::Lit(1)),
```

Same default in the Lua loader:
<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-lua/src/npc_dialogue.rs" lines="287-310" />

So `DeleteMoney` correctly charges `Price = 50`, and `Create` gives 1. Exact match to the
report.

### Fix

1. `lower.rs` — change both defaults to `DialogueExpr::Session(SessionVar::Amount)`.
2. `npc_dialogue.rs` — same default for the `Value::Nil` arm (keeps hand-authored TFS-style
   scripts that omit `count` on the 772 contract).
3. `emit.rs` — when `count == Session(Amount)`, emit `{ create = { item = ... } }` with no
   `count` key, mirroring the existing `createMoney = true` / `deleteMoney = true` shorthand.
   Keeps the generated pack readable and self-documenting.
4. Keep the two-arg `Create(a, b)` extension for new TFS content (the 772 grammar has none;
   the only two-arg occurrences in `runtime/npc/` are commented out in `chrystal.npc`).

---

## 3. CRITICAL — `create_item` does not loop non-cumulative types

Blank rune is **not** stackable in 772 (`runtime/dat/objects.srv` TypeID 3147:
`Flags = {Take,Special}` — no `Cumulative`), so even with #2 fixed the player would still get
one item.

### Reference behavior

```c
// crnonpl.cc:1870-1895
void TNPC::GiveTo(ObjectType Type, int Amount){
    if(Amount == 0) return;
    if(Type.getFlag(CUMULATIVE)){
        while(Amount > 0){ int S = std::min(Amount,100); CreateAtCreature(Interlocutor,Type,S); Amount -= S; }
    }else{
        while(Amount > 0){ CreateAtCreature(Interlocutor, Type, this->Data); Amount -= 1; }
    }
}
```

Two distinct loops: cumulative → stacks of ≤100; non-cumulative → **N separate objects**,
each created with `Data` as the subtype.

### Rust behavior

<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/player/inventory/money.rs" lines="74-95" />

`player_add_item_count` chunks by 100 regardless of stackability and hands `chunk` to
`lua_script_player_add_item_full`, which for a non-stackable clamps it into a single item's
`count` field:

<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/game_world_inventory.rs" lines="421-435" />

Result: `player_add_item_count(cid, 2260, 5)` → **one** blank rune with `count = 5`.

### Fix

Split the two arms in `player_add_item_count` (or better, add a dedicated
`npc_give_to(player, item_id, amount, data)` in `npc/host.rs` that cites `TNPC::GiveTo` and
leaves `player_add_item_count` as the coin-only helper):

```rust
if self.items_db.items.get(&item_id).is_some_and(|t| t.stackable()) {
    let mut remaining = count;
    while remaining > 0 { let chunk = remaining.min(100); /* add stack */ remaining -= chunk; }
} else {
    for _ in 0..count { /* add one item with sub_type = data */ }
}
```

Mirror it for `delete_item` against `DeleteAtCreature` (`operate.cc:2728-2751`), which also
branches on `CUMULATIVE` (partial-stack `Change(AMOUNT)` vs whole-object `Delete`).

**Sanity guard:** `Amount` comes from a `%1` capture capped at 500 (`numeric_capture_cap`), so
the worst case is 500 iterations — but add a hard cap and a `tracing::warn` so a malformed
behaviour file can't allocate unbounded items.

---

## 4. HIGH — Generated data pack must be regenerated

623 `count = 1` sites across 195 files in `data/npc/scripts/`. Every 772 shop in the pack
currently sells exactly one unit. Example (`data/npc/scripts/xodet.lua:344-345`):

```lua
{ deleteMoney = true },
{ create = { item = { session = "type" }, count = 1 } },
```

After the #2 fix, regenerate:

```bash
cargo run -p tfs-rust-lua --bin import-npcs -- \
  --root reference/cipsoft-772/runtime/npc \
  --out data/npc/scripts \
  --validate-data-dir data \
  --keep-extra
```

Review the diff for hand-edited scripts before committing — `--keep-extra` preserves files
that were not overwritten, but any manual edit to a *generated* file is lost.

---

## 5. HIGH — `Npc->Data` (subtype) is dropped by Create / Delete / Count

`Data` is the fluid type / key number selector in 772:

| Call | C++ | Rust |
|---|---|---|
| `Create(x)` non-cumulative | `CreateAtCreature(..., this->Data)` (`crnonpl.cc:1891`) | `create_item(player, id, count)` — no `data` |
| `Delete(x)` | `DeleteAtCreature(..., Amount, this->Data)` (`crnonpl.cc:1900`) | `player_remove_item_of_type(..., sub_type = -1)` |
| `Count(x)` | `CountInventoryObjects(..., Npc->Data)` (`crnonpl.cc:790`) — filters on `CONTAINERLIQUIDTYPE` / `KEYNUMBER` | `player_get_item_type_count(player, id, -1)` |

Concrete breakage in Xodet: `"life","fluid" -> Type=2874, Data=11` sells a vial that must
carry fluid type 11. Rust creates a plain empty vial. The `"deposit"` branch
(`Data=0`, `Count(2874)`) is supposed to count only *empty* vials and instead counts every
vial including full potions.

**Fix:** thread `data` through `NpcActionHost`:

```rust
fn create_item(&mut self, player: CreatureId, item_id: i32, count: i32, data: i32) -> Result<(), String>;
fn delete_item(&mut self, player: CreatureId, item_id: i32, count: i32, data: i32) -> Result<(), String>;
```

and change the `inventory_count` closure in `focus.rs` to capture the session `data` and pass
it as `sub_type` when the type is a fluid container / key
(`ItemDatabase::is_fluid_container` already exists in `crates/tfs-rust-content/src/otb.rs:607`).
Note the C++ quirk worth replicating for parity: `CountObjects` **does not** propagate `Value`
into nested containers (`info.cc:553`, flagged `BUG(fusion)` in the decompile) — cite it
explicitly rather than silently "fixing" it.

---

## 6. HIGH — Gold duplication via `Delete` + `CreateMoney`

Consequence of #2/#3, called out separately because it is an economy exploit rather than a
missing item. Xodet `Topic=3`:

```
Topic=3,"yes",Count(2874)>0 -> Amount=Count(2874), Price=Amount*5, "Here you are ... %P gold.", Delete(2874), CreateMoney
```

`CreateMoney` (bare) correctly lowers to `Session(Amount)`
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-content/src/npc_import/lower.rs" lines="186-194" />)
so the player is paid for **all** vials, while `Delete(2874)` removes **one**. Repeatable
money printer. Fixed by #2 + #3; add a regression test specifically for this shape.

Note the asymmetry is intentional in 772 and must be preserved: bare `DeleteMoney` uses
`Price`, bare `CreateMoney` uses `Amount` (`crnonpl.cc:1190-1191`). Current lowering already
matches — do not "normalize" it.

---

## 7. MED — `%T` is wrong twice over

```rust
// focus.rs:778-783
let now = chrono::Local::now();
let h = now.hour() % 12;
((if h == 0 { 12 } else { h }) as u8, now.minute() as u8)

// expr.rs:145-151
fn format_game_time(hour: u8, minute: u8) -> String {
    if hour < 12 { format!("{hour}:{minute:02} am") } else { ... }
}
```

Two defects:

1. **Every PM time prints "am".** `game_hour` is pre-normalized to 1..12, so `hour < 12` is
   true for 1 pm … 11 pm. Only 12 takes the pm branch, and it renders as `"0:00 pm"`.
2. **Wrong clock.** 772 game time is one game *day* per real hour:

```c
// time.cc:43-49
int Time = LocalTime.tm_sec + LocalTime.tm_min * 60;
*Hour   = (Time / 150);
*Minute = (Time % 150) * 2 / 5;
```

Rust uses real wall-clock hour/minute.

**Fix:** `crates/tfs-rust-core/src/world_light.rs:6` already computes exactly this value
(`world_time_from_local_clock()` → 0..1439 game minutes, same `/2.5` derivation used for
ambient light). Feed `game_hour = wt / 60`, `game_minute = wt % 60` (raw 0..23 hour), and
restore the C++ branch verbatim:

```rust
if hour < 12 { format!("{hour}:{minute:02} am") } else { format!("{}:{minute:02} pm", hour - 12) }
```

This also removes a second wall-clock source of truth from the codebase.

---

## 8. MED — `ToDoClear` drops pending NPC `ChangeState`

```c
// cract.cc:970-976
case TDChangeState:{
    if(this->ActToDo <= i && this->Type == NPC){
        ChangeNPCState(this, TD->ChangeState.NewState, false);
    }
    break;
}
```

C++ **applies** any not-yet-executed `TDChangeState` while clearing. Rust's
`player_todo_clear` just wipes the queue:

<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/walk/mod.rs" lines="664-686" />

`apply_dialogue_plan` sets `deferred_idle` → activity `Leaving` + a queued
`ChangeNpcState { to_idle: true }` (`crnonpl.cc:1219-1222`). If that entry is cleared before
it runs, the NPC is stuck in `Leaving`: `npc_idle_stimulus` only handles `Talking` and `Idle`,
the queue drain requires `Idle`, and sleep/roam requires `Idle` — the NPC goes permanently
dormant. This becomes *much* more reachable once finding #1 is fixed (clears get frequent).

**Fix:** apply the pending state transition inside the clear, before draining the queue:

```rust
let pending_idle = base.todo.queue.iter()
    .any(|a| matches!(a, CreatureAction::ChangeNpcState { to_idle: true }));
// ... after clearing, if pending_idle && creature is an NPC: set Idle + focus = None
```

Also rename the helper — `player_todo_clear` is called on NPCs and monsters throughout; it is
`TCreature::ToDoClear`, not a player-specific function.

---

## 9. MED — VANISH transition skips `ToDoYield`

```c
// crnonpl.cc:1850-1855
this->Behaviour->react(this, "", VANISH);
this->ChangeState(IDLE, true);   // Stimulus = true → ToDoYield()
```

`ChangeState(_, true)` calls `ToDoYield()` (`crnonpl.cc:1973-1978`), which arms
`ToDoWait(0)` + `ToDoStart()` when not locked — restarting the idle/roam loop immediately.

`npc_creature_move_stimulus` sets `activity = Idle` with no yield:
<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/npc/focus.rs" lines="451-459" />

The sleeping-wake arm on the same function does it correctly
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/npc/focus.rs" lines="416-424" />),
so this is an inconsistency, not a missing capability.

**Fix:** call `self.creature_todo_yield(npc_id)` after the state flip. Same for the
`Interlocutor == NULL` arm (C++ `crnonpl.cc:1841-1844` also yields).

---

## 10. MED — `%1` / `%2` capture lifetime

C++ declares `int Parameters[2] = {-1,-1}` **once, outside** the behaviour loop
(`crnonpl.cc:995`) and never resets it. A capture written by a rule that later fails to match
stays visible to subsequent rules' `BEHAVIOUR_CONDITION_EXPRESSION` evaluation and to the
winning rule's action phase.

Rust allocates a fresh `MatchCaptures::default()` per rule:
<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/npc/match_rule.rs" lines="73-77" />

This is *cleaner* but not parity. It matters for behaviour files where an early
`%1,...` rule partially matches and a later rule references `%1` without re-capturing it —
C++ reads the stale value, Rust reads `-1`.

**Fix:** hoist `captures` out of the per-rule loop in `match_dialogue_rule_with_custom` and
carry the accumulated array into `RuleMatch`. Add a comment citing `crnonpl.cc:995` so nobody
"fixes" it back. **Verify against real behaviour files first** — if no shipped `.npc` depends
on the leak, prefer documenting the deliberate divergence over adopting the quirk.

---

## 11. LOW — `Idle` action under `ADDRESSQUEUE`

```c
// crnonpl.cc:1210-1224
if(!StartToDo){
    Npc->ChangeState(NewState, false);
    if(Situation != ADDRESSQUEUE){ StartToDo = true; }
    else { error("NPC %s reagiert nicht auf Anrede %s.\n", ...); }
}else{
    if(NewState == IDLE){ Npc->ChangeState(LEAVING, false); }
    Npc->ToDoChangeState(NewState);
}
```

Rust unconditionally sets `plan.start_todo = true`:
<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/npc/react.rs" lines="122-131" />

Under `ADDRESSQUEUE` this adds a trailing `Wait` + `ToDoStart` that C++ omits (C++ treats it
as a content error). Low impact — reachable only when a queued address matches an
`Idle`-only rule with no preceding `Say`.

**Fix:** gate on `situation != DialogueSituationKind::AddressQueue` and `tracing::warn!` on
the error path, matching the decompile.

---

## 12/13. LOW — `FormatNpcResponse` edge cases

Reference (`crnonpl.cc:899-974`):

- Recognizes **uppercase only**: `N`, `A`, `P`, `T`.
- Unknown `%X`: `Help` stays empty, `ReadPos += 2` → **both characters are consumed and
  nothing is emitted**.
- Buffer is `char[256]`; on overflow `FormatNpcResponse` returns `false` and the caller
  **drops the reply entirely** and logs an error (`crnonpl.cc:1116-1119`) — no `ToDoTalk`, no
  `TalkDelay` bump, no `StartToDo`.

Rust (<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/npc/expr.rs" lines="120-143" />)
accepts lowercase, passes unknown `%X` through verbatim, and has no length cap.

**Fix:** match the C++ table exactly (uppercase only, consume-and-drop unknown escapes), and
add the 255-byte cap in `apply_dialogue_plan`'s `Say` arm — skip the reply + `tracing::warn`
instead of pushing a `PlannedReply`. The cap also matters for wire parity: an over-long NPC
line would otherwise be broadcast where 772 stays silent.

---

## 14. LOW/SUSPECT — sleeping wake gated on `focus.is_none()`

C++ wakes a sleeping NPC on any nearby non-delete creature move regardless of `Interlocutor`
(`crnonpl.cc:1863-1867`); `ChangeState(IDLE)` never clears `Interlocutor`. Rust returns early
when `focus.is_some()` (<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/npc/focus.rs" lines="417-426" />).

In practice Rust clears `focus` on every Idle transition, so the states should not co-occur —
but finding #8 creates exactly such a state (`Leaving` with focus set, then later forced
Idle/Sleeping). Worth a targeted check after #8 lands; likely resolves itself.

---

## 15. LOW/GAP — `GetFrom` error semantics

`DeleteAtCreature` `throw ERROR` when it cannot find enough objects (`operate.cc:2732-2735`).
`react` does not catch it, so the exception unwinds out of the whole reaction — remaining
actions in the rule are skipped. Rust logs via `log_action_failure` and continues
(<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/npc/react.rs" lines="155-171" />).

Only reachable when a behaviour file's `Count(...)` guard disagrees with the actual removal —
i.e. exactly the situation finding #5 creates. Low priority once #5 is fixed, but the
"abort remaining actions on delete failure" shape is the safer economy behavior and should be
adopted: return a sentinel from the `Delete` arm and `break` the action loop.

---

## Confirmed-correct (checked, no action)

These were compared and match the reference — recorded so the next audit doesn't re-derive
them:

- **Reply timing constants.** `NpcTuning::classic_772()` — initial 1000 ms, base 3100 ms,
  100 ms per 2 bytes, keepalive 2000 ms, timeout 30 rounds, focus box `<5 / <4`, sleep search
  10×10, roam 10 attempts / 2000 ms delay. All match `crnonpl.cc:1087-1113`, `:1720-1722`,
  `:1762-1804`, `:1821`.
- **`LastTalk = TalkDelay/1000 + RoundNr`**, skipped under `BUSY` (`crnonpl.cc:1287-1289`).
- **`TalkStimulus` engagement test** — `State == TALKING || QueueLength != 0`, BUSY swaps
  `Interlocutor` around the reaction and restores it (`crnonpl.cc:1690-1700`).
- **`Enqueue` dedupe** by player id, ordered FIFO removal in `IdleStimulus`
  (`crnonpl.cc:1937-1961`, `:1735-1745`).
- **Speech fan-out** — `TFindCreatures(3,3,…,FIND_NPCS)`, same-floor, `TALK_SAY` from players
  only (`operate.cc:2451-2468`).
- **`SearchForWord` / `SearchForNumber`** including the `$` whole-word suffix and the
  `TextPtr = Parameter + 1` single-character advance quirk (`strings.cc:318-407`,
  `crnonpl.cc:1019`, `:1044`).
- **Rule selection** — highest condition count wins, `!` short-circuits, `Topic = 0` reset on
  non-BUSY situations (`crnonpl.cc:1068-1082`).
- **`MovePossible`** — BANK / UNPASS / AVOID / house / radius / same-z (`crnonpl.cc:1672-1679`).
- **`CalculateChange`** money denomination + change-making (`info.cc:634-687`).
- **`ToDoStart` delay clamp** `max(1)` and absolute `Wait` deadline handling in the drain
  (`cract.cc:1010-1024`, `:905-915`).

---

## Fix plan

### Phase 1 — user-reported defects (ship together)

| Step | Change | Files | Status |
|---|---|---|---|
| 1.1 | `creature_todo_add` helper with the `ToDoAdd` clear preamble; route NPC dialogue enqueues through it | `creature_todo.rs`, `npc/focus.rs` | ✅ Done |
| 1.2 | Apply pending `ChangeNpcState` inside `ToDoClear` (prevents `Leaving` deadlock once 1.1 makes clears frequent) | `walk/mod.rs` | ✅ Done |
| 1.3 | `Create`/`Delete` default count → `Session(Amount)` | `npc_import/lower.rs`, `tfs-rust-lua/src/npc_dialogue.rs` | ✅ Done |
| 1.4 | Emit shorthand (omit `count` when it is `Session(Amount)`) | `npc_import/emit.rs` | ✅ Done |
| 1.5 | Split cumulative / non-cumulative loops in the give path | `player/inventory/money.rs` or new `npc/host.rs` helper | ✅ Done (`npc/host.rs`) |
| 1.6 | Mirror the split in the delete path (`DeleteAtCreature` parity) | `game_world_inventory.rs` | ✅ Done (`npc/host.rs`) |
| 1.7 | Regenerate `data/npc/scripts/*.lua` | `import-npcs` | ✅ Done |

**Order matters:** 1.2 before 1.1 (otherwise 1.1 introduces the `Leaving` deadlock), and 1.3/1.4
before 1.7.

### Phase 2 — correctness follow-ups

| Step | Change | Files |
|---|---|---|
| 2.1 | Thread `Npc->Data` into Create / Delete / `Count(...)` | `npc/actions.rs`, `npc/host.rs`, `npc/focus.rs` |
| 2.2 | `%T` → 772 game time via `world_time_from_local_clock()`, fix am/pm | `npc/expr.rs`, `npc/focus.rs` |
| 2.3 | `ToDoYield` on the VANISH / null-interlocutor Idle transitions | `npc/focus.rs` |
| 2.4 | `Idle` action: don't set `StartToDo` under `ADDRESSQUEUE` | `npc/react.rs` |
| 2.5 | `Delete` failure aborts remaining actions in the rule | `npc/react.rs` |

### Phase 3 — parity polish

| Step | Change | Files |
|---|---|---|
| 3.1 | Decide + document `%1`/`%2` capture lifetime (adopt the shared array or record the divergence) | `npc/match_rule.rs` |
| 3.2 | `FormatNpcResponse` escape table + 255-byte drop rule | `npc/expr.rs`, `npc/react.rs` |
| 3.3 | Phase-B rollout of `creature_todo_add` to monster / player call sites; delete the now-redundant explicit `player_todo_clear_with_snapback` preambles | `idle_stimulus.rs`, `game_loop.rs` |
| 3.4 | Rename `player_todo_clear` → `creature_todo_clear` (it is `TCreature::ToDoClear`) | repo-wide |

---

## Tests to add

`crates/tfs-rust-core/src/npc/tests.rs`:

1. ✅ Added: `talk_interrupt_clears_pending_replies` — ADDRESS at T, DEFAULT at T+1500; assert the
   queue holds exactly the new reaction's entries and the reply deadline is `T+2500`,
   not `T+5900`. **This is the direct regression test for user report #1.**
2. `create_uses_amount_session_var` — `Amount = 5`, `Create(Type)` → `create_item` called
   with count 5. (covered by `create_non_cumulative_spawns_n_items`)
3. ✅ Added: `create_non_cumulative_spawns_n_items` — 5 blank runes → 5 distinct `ItemId`s each with
   `count == 1`.
4. ✅ Added: `create_cumulative_chunks_by_100` — 250 gold coins → stacks 100/100/50.
5. ✅ Added: `vial_deposit_no_money_duplication` — Xodet `Topic=3` shape: N vials in, gold paid
   equals vial count, all N vials gone; assert no duplication.
6. ✅ Added: `deferred_idle_survives_todo_clear` — `Leaving` + pending `ChangeNpcState`, then clear;
   assert the NPC lands in `Idle`, not `Leaving`.
7. `format_time_pm` — game hour 13 → `"1:00 pm"` (*Phase 2*).
8. `create_respects_data_subtype` — `Data = 11`, `Type = 2874` → vial with fluid type 11
   (*Phase 2*).

`crates/tfs-rust-content/src/npc_import/lower.rs` unit tests: ✅ Added
`create_delete_default_count_to_session_amount` to assert `Create(2260)` / `Delete(2260)` lower to
`count: Session(Amount)` when no explicit count is supplied.

---

## Verification

```bash
rtk cargo check --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo test -p tfs-rust-core npc::
rtk cargo test -p tfs-rust-content npc_import::
rtk cargo test --workspace

# after the importer change
cargo run -p tfs-rust-lua --bin import-npcs -- \
  --root reference/cipsoft-772/runtime/npc \
  --out data/npc/scripts --validate-data-dir data --keep-extra
rtk git diff --stat data/npc/scripts
```

**Phase 1 verification results:**
- `cargo check --workspace` passes.
- `cargo clippy` passes for touched crates (pre-existing warnings remain elsewhere).
- `cargo test -p tfs-rust-core npc::` — 29 passed.
- `cargo test -p tfs-rust-content` — 90 passed.
- `cargo test -p tfs-rust-core` shows 3 unrelated pre-existing failures:
  `container_ui::tests::ground_container_stays_open_when_player_adjacent`,
  `monster_ai::world_tests::active_monster_random_roams_after_one_second`,
  `player::combat::ranged::tests::wand_fire_animated_text_uses_orange_not_blood_red`.

In-game smoke test against Xodet (`[32399,32222,7]`):

1. `hi` → `blank rune` → `yes` — 1 rune, 10 gp.
2. `hi` → `5 blank runes` → `yes` — **5 runes**, 50 gp.
3. `hi` → `blank rune`, then immediately `spellbook` before the first reply lands — the first
   reply is dropped and the spellbook reply arrives ~1 s after the second message.
4. `hi` → `mana fluid` → `yes` — vial carries fluid type 10 (Phase 2).
5. `deposit` → `yes` with N empty vials — all N removed, `N*5` gp paid.
