# Movement: C++ TFS vs Australis Rust — gap analysis

Purpose: explain why movement can feel like **teleporting / glitching** in the Rust port, and what would be required for **behavioral parity** with TFS 1.4.2-style logic in this repo.

**Sources:** `src/game.cpp` (`Game::playerMove`, `internalMoveCreature`, `checkCreatureWalk`), `src/creature.cpp` (`startAutoWalk`, `addEventWalk`), `src/protocolgame.cpp` (`sendMoveCreature`, walk opcodes), `crates/tfs-rust-core/src/game_loop.rs` (`handle_player_move`), `crates/tfs-rust-core/src/tile.rs` (`query_add`).

---

## 1. Critical: walk is **scheduled**, not instant (C++)

### C++

1. `ProtocolGame` receives walk opcode `0x65`–`0x68` / diagonals → `addGameTask(&Game::playerMove, playerId, direction)` (`protocolgame.cpp` ~556–564).
2. `Game::playerMove` **does not** move the creature on the map (`game.cpp` ~1880–1896). It calls:
   - `player->isMovementBlocked()` → may `sendCancelWalk()` and return;
   - `player->startAutoWalk(direction)`;
3. `Creature::startAutoWalk(Direction)` (`creature.cpp` ~274–285):
   - Clears `listWalkDir`, pushes **one** direction;
   - Calls `addEventWalk(true)`.
4. `addEventWalk` (`creature.cpp` ~299–322):
   - Computes delay: `ticks = getEventStepTicks(firstStep)` (derived from **speed**);
   - Schedules `Game::checkCreatureWalk(creatureId)` on the **scheduler** after `ticks`.
5. `Game::checkCreatureWalk` (`game.cpp` ~3773–3780) calls `creature->onWalk()`, which eventually performs **`internalMoveCreature`** and only then notifies protocol / spectators (`sendMoveCreature`, etc.).

So: **one keypress → one scheduled step**, aligned with **step speed**, not “move every network packet instantly.”

### Rust (`game_loop.rs` ~120–173)

- Resolves `conn_id` → player, computes `new_pos = old_pos.offset(dir)`.
- **Immediately** updates `pl.base.position`, map creature index, and builds **`send_move_creature_player`** → `enqueue_outgoing`.
- **No** scheduler delay, **no** `getEventStepTicks`, **no** `checkCreatureWalk` / `onWalk` equivalent.

**Gap:** Server applies **one full tile per input event** with **no timing** tied to speed. OTClient still runs **local walk animation** at its own pace; when the next server packet arrives, the position can **jump** relative to what the client predicted → **teleport / rubber-band** feel.

---

## 2. Movement validation (C++ yes, Rust stub)

### C++

`Game::internalMoveCreature(Creature, Direction)` (`game.cpp` ~797–841):

- Computes `destPos` from direction (plus **stairs / height / floor-change** logic for players).
- Gets `toTile`; calls `internalMoveCreature(creature, toTile, flags)`.
- `internalMoveCreature(Creature&, Tile&, flags)` (~843+): `toTile.queryAdd(...)`, `map.moveCreature`, etc. On failure, returns `ReturnValue` and player may get **cancel messages**.

### Rust (`tile.rs` ~63–66)

```rust
pub fn query_add(&self, _thing_size: u8) -> bool {
    // ... minimal placeholder
    true
}
```

**Gap:** **No** real blocking, items, creatures, or zone checks. Even if timing were fixed, **collision parity** is missing.

---

## 3. Block / cancel walk (C++)

- `player->isMovementBlocked()` before starting walk (`game.cpp` / `creature.cpp`).
- Failure paths: `sendCancelWalk`, `sendCancelMessage`, etc.

### Rust

- No `sendCancelWalk` (or equivalent) on failed move.
- No condition / drunk / exhaustion checks.

**Gap:** Client never gets **official** walk cancellation aligned with TFS.

---

## 4. Auto-walk and path (C++)

- `parseAutoWalk` (`0x64`) builds a **path** → `Game::playerAutoWalk` → `startAutoWalk(listDir)` (`game.cpp` ~2075+).
- `playerStopAutoWalk` (`0x69`).
- Queue: `listWalkDir` processed over multiple scheduler steps.

### Rust

- No `0x64` auto-walk parsing in the game simulation (only whatever the net layer parses).
- No multi-step queue for cardinal key spam beyond “one move per packet.”

**Gap:** **Auto-walk** and **queued directions** are absent.

---

## 5. Protocol: `sendMoveCreature` shape (mostly aligned)

`ProtocolGame::sendMoveCreature` for **local player**, non-teleport (`protocolgame.cpp` ~2827–2870): `0x6D` + old pos/stack + new pos, then optional floor moves, then **map row opcodes** `0x65`/`0x67`/`0x66`/`0x68` with `GetMapDescription` strips.

Rust **`send_move_creature_player`** (`map_description.rs`) mirrors this structure for same-floor and z-change branches.

**Gap (secondary):** If **timing** or **known-creature set** is wrong, strips can still desync; primary issue for “glitch” is usually **§1** + **§2**, not missing opcode bytes.

---

## 6. Summary table

| Area | C++ (TFS) | Rust (current) |
|------|-----------|----------------|
| Walk input → map change | Delayed via **scheduler** + `onWalk` / `internalMoveCreature` | **Immediate** on packet |
| Step interval | `getEventStepTicks` / speed | None |
| Tile eligibility | `queryAdd`, flags, stairs logic | `query_add` → always `true` |
| Blocked movement | `sendCancelWalk` / messages | Not implemented |
| Auto-walk `0x64` | Path + `playerAutoWalk` | Not wired to sim |
| `sendMoveCreature` | After successful move | After every accepted move packet |

---

## 7. Recommended direction (for parity)

1. **Minimum viable “smooth” feel:** introduce a **walk step queue** and **per-player next-walk tick** (or reuse a small scheduler) so **at most one tile step per N ms** derived from base speed / TFS `getEventStepTicks` semantics — still call `send_move_creature_player` only when the step **commits**.
2. **Correctness:** replace `Tile::query_add` stub with real **`queryAdd`-style** checks (blocking items, creatures, optional flags) before committing a step.
3. **Full parity:** port **`Creature::onWalk` → `internalMoveCreature(Direction)`** flow including floor-change edge cases from `game.cpp` ~797–834.

This document is descriptive only; implementation should follow `tasks/todo.md` / project workflow when you start coding.
