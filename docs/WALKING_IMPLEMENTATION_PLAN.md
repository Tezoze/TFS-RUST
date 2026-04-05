# Plan: Full TFS 1.4.2–style walking (scheduled steps + validation)

This document is an **implementation roadmap** for movement parity with The Forgotten Server (TFS) 1.4.2. It complements the gap analysis in [`MOVEMENT_CPP_VS_RUST.md`](./MOVEMENT_CPP_VS_RUST.md).

**Non-negotiable:** Behavior, packet ordering, and failure semantics must match C++ unless a documented deviation is approved (see project `.cursorrules`).

---

## 1. Goals and success criteria

| Goal | Verification |
|------|----------------|
| One **committed** tile step per **scheduler interval** derived from speed (TFS `getEventStepTicks`), not one tile per raw walk opcode | Client prediction stays aligned; no “teleport” feel from instant server position updates |
| Walk input **queues** directions (`listWalkDir` equivalent) and **auto-walk `0x64`** feeds the same queue | Long paths drain over multiple ticks |
| **Blocked** or **invalid** steps: no silent success — `sendCancelWalk` / cancel messages as in C++ | Compare bytes and client UI with TFS on walls, exhaustion, etc. |
| **`internalMoveCreature` + `Tile::queryAdd`** parity for normal moves | Same outcomes as `game.cpp` / `tile.cpp` for blocking, creatures, height |

**Out of scope for “full walking” v1 (track separately):** swimming, mounts, special tiles (magic fields), every floor-change edge case — port in order of C++ call graph once base path works.

---

## 2. C++ source of truth (read order)

Implementers should keep these files open while coding:

| Concern | TFS 1.4.2 file / symbol |
|---------|-------------------------|
| Walk opcode → game task | `protocolgame.cpp`: `parsePacket` walk / `playerMove` dispatch |
| Player initiates walk | `game.cpp`: `Game::playerMove`, `Game::playerAutoWalk`, `Game::playerStopAutoWalk` |
| Scheduler step | `game.cpp`: `Game::checkCreatureWalk` |
| Step timing & queue | `creature.cpp`: `Creature::startAutoWalk`, `addEventWalk`, `getEventStepTicks`, `onWalk`, `goToFollowCreature` (if following shares queue) |
| Actual position change | `game.cpp`: `Game::internalMoveCreature` (direction overload + tile overload) |
| Tile rules | `tile.cpp`: `Tile::queryAdd`, height/stack rules used by movement |
| Wire format | `protocolgame.cpp`: `sendMoveCreature`, `sendCancelWalk` |

Rust already mirrors **`send_move_creature_player`** and **`send_cancel_walk`** (`tfs-rust-net`); the **simulation** must call them at the same logical points as C++.

---

## 3. Current Rust touchpoints

| Location | Today | Target |
|----------|-------|--------|
| `crates/tfs-rust-core/src/game_loop.rs` | `handle_player_move` applies move **immediately** | Thin handler: enqueue direction / merge with queue; **no** map commit here |
| `crates/tfs-rust-core/src/game_world.rs` | `on_tick` has no walk phase | Call **`process_creature_walk`** (or equivalent) **each tick**, before/after order per TFS `gameLoop` |
| `crates/tfs-rust-core/src/creature/base.rs` | `walk_queue: VecDeque<Direction>` unused for players | Holds **`listWalkDir`**; optional **`next_walk_tick`** or event id |
| `crates/tfs-rust-core/src/tile.rs` | `query_add` → always `true` | Real checks using item DB + tile flags + creature stack |
| `crates/tfs-rust-net` | `GamePacket::AutoWalk`, `StopAutoWalk`, `send_cancel_walk` exist | Wire `AutoWalk` / `StopAutoWalk` in `game_loop` to same queue as single-step walks |

---

## 4. Architecture (Rust-friendly, TFS-equivalent)

### 4.1 Time model

TFS uses a **scheduler** with **tick** delays (`getEventStepTicks`). The Rust engine already has **`tick_counter`** in `GameWorld` (~50 ms per tick via Tokio).

**Decision (to confirm against C++):**

- Either map **`getEventStepTicks`** to **“execute on tick `T + k`”** where `k` is derived from speed (integer math matching `creature.cpp`),  
- Or run a **small per-creature deadline** (`next_step_at_tick: u64`) updated after each successful step.

Document the **exact formula** next to the ported C++ function (mirror `Creature::getStepDuration` / step ticks in 1.4.2).

### 4.2 State machine (per creature with walk queue)

1. **Input path:** `playerMove(direction)` equivalent  
   - If movement blocked → `sendCancelWalk` + return (see §5).  
   - Else `startAutoWalk(direction)` → clear or append to queue per C++ (`startAutoWalk` semantics: single key vs queue).  
   - Schedule **first** `checkCreatureWalk` if not already walking.

2. **Tick path:** `checkCreatureWalk` equivalent  
   - If queue empty → nothing.  
   - If not yet time for next step (per step ticks) → reschedule; **do not** pop queue early.  
   - Else pop one direction, run **`internalMoveCreature`**; on success emit **`sendMoveCreature`** / spectator updates; on failure clear queue / cancel as C++.

3. **Auto-walk:** `playerAutoWalk` feeds **multiple** directions into the **same** queue as manual steps (after `parseAutoWalk` — net layer already reverses path to match `getPreviousByte` behavior).

### 4.3 Borrowing / structure

Avoid `Arc<RwLock<Player>>`. Preferred pattern:

- **`GameWorld::process_creature_walk`** takes `&mut self` and iterates creatures with walk state, or  
- Split: **`WalkController`** struct holding only **`CreatureId` → walk state** updated inside one `&mut GameWorld` pass.

Use **`CreatureId`** keys; no raw pointers.

---

## 5. Implementation phases

### Phase A — Walk state + scheduling (no full `query_add` yet)

**Deliverables**

- [ ] Add fields (on `CreatureBase` or a dedicated `WalkState` component): e.g. **`walk_queue`**, **`walking` flag**, **`next_walk_game_tick`** (or scheduler handle), matching **`listWalkDir` + event** semantics.
- [ ] Implement **`get_event_step_ticks`** (name as you prefer) from **`Creature::getEventStepTicks`** / step duration in TFS 1.4.2 — **numeric parity** with C++.
- [ ] Replace **`handle_player_move`** body: push direction per **`startAutoWalk`** rules; **do not** update `position` or send `0x6D` here.
- [ ] Implement **`GameWorld::process_creature_walk`** (or `check_creature_walk` on `GameWorld`) called from **`on_tick`**, performing **at most one** step per creature per scheduled slot.
- [ ] On successful step only: update map index + position + **`send_move_creature_player`** (existing helper).

**Tests**

- Unit test: given fixed speed, step happens only after **`N` ticks**, not on opcode receipt.
- Integration: two rapid walk packets → **two tiles** over **two intervals**, not two tiles in one tick.

### Phase B — `internalMoveCreature` + destination resolution

**Deliverables**

- [ ] Port **`Game::internalMoveCreature(Creature*, Direction)`** logic: compute **`destPos`**, handle **stairs / floor change** as in `game.cpp` (~797–834) for the subset you support.
- [ ] Single entry point **`try_move_creature(creature_id, direction) -> MoveResult`** used by the walk tick.
- [ ] On **`RETURNVALUE_NOTPOSSIBLE`** (or equivalent): **no** position change; **`send_cancel_walk`** with correct **facing direction**; optionally cancel messages — match `game.cpp` / `player.cpp` branches.

**Tests**

- Walk into unwalkable offset → position unchanged, cancel packet emitted once.

### Phase C — `Tile::query_add` and collision parity

**Deliverables**

- [ ] Replace stub: **`Tile::query_add`** uses **item attributes** from **`items_db`**, **tile flags**, **existing creatures** on destination tile, **thing size** — mirror **`Tile::queryAdd`** in `tile.cpp` for supported cases.
- [ ] Wire **`query_add`** from **`internalMoveCreature`** before **`map.moveCreature`** equivalent.

**Tests**

- Tile with blocking item → step fails, **`query_add` false**, cancel walk.
- Tile occupied when not walk-through → same.

### Phase D — Block conditions (`isMovementBlocked`)

**Deliverables**

- [ ] Port **`Player::isMovementBlocked`** (and creature analogues): conditions (root, drunk, etc.), **PZ** rules if applicable, **cap** exhaustion — strictly from TFS 1.4.2.
- [ ] Call from **`playerMove`** **before** queueing; **`sendCancelWalk`** when blocked.

### Phase E — Auto-walk and stop

**Deliverables**

- [ ] In **`game_loop`**, handle **`GamePacket::AutoWalk { path }`**: call **`playerAutoWalk`** equivalent — load **`walk_queue`** from path (net already orders dirs).
- [ ] Handle **`GamePacket::StopAutoWalk`**: clear queue and/or send cancel per C++ **`playerStopAutoWalk`**.

### Phase F — Spectators and multi-client consistency

**Deliverables**

- [ ] Ensure **`sendMoveCreature`** for **other** viewers matches TFS ordering (local vs spectator). May require **`send_move_creature`** variants already in `tfs-rust-net` — audit vs `protocolgame.cpp`.
- [ ] **`known_creatures_by_conn`** updates stay consistent when move is **scheduled** (strip packets only on **committed** step).

---

## 6. Ordering inside `on_tick`

Lock order to C++ **`Game::gameLoop`** (verify line order in `game.cpp`):

1. Dispatcher / scheduled events (if you fold walk into “events”, run **checkCreatureWalk** in the same relative order as TFS).
2. Other creature ticks (monsters, etc.) — **confirm** whether walk should run before or after monster think; match C++.

Document the chosen order in code comments with **file:line** reference.

---

## 7. Verification checklist (manual + automated)

- [ ] OTClient 10.98: hold forward — smooth walk, no rubber-band (compare with same client against TFS).
- [ ] Walk into wall — **`0xB5`** cancel with correct direction byte.
- [ ] Auto-walk long click — path drains over time; **stop** mid-path clears correctly.
- [ ] `cargo test` for movement unit tests; optional **replay** test from logged opcodes.

---

## 8. Traceability

Each new function should cite **TFS 1.4.2** reference symbol in a one-line comment, e.g.:

`// C++: Game::internalMoveCreature (game.cpp ~797)`

---

## 9. Related documents

- [`MOVEMENT_CPP_VS_RUST.md`](./MOVEMENT_CPP_VS_RUST.md) — gap analysis  
- [`OTCLIENT_INFO.md`](./OTCLIENT_INFO.md) — client protocol notes  
- Project rules: `.cursorrules` (compatibility mandate)
