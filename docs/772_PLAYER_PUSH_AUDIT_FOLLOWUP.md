# 772 Player Push — Follow-up Audit (Chain-Push Focus)

**Audited:** 2026-08-09
**Scope:** Re-audit of the player-push path against the 772 decompile, focused on the
chain-push behavior (pushing a creature onto a tile occupied by another creature, which is
in turn shoved further — "X - X - X").
**Predecessor:** `docs/772_PLAYER_PUSH_AUDIT.md` (P-A through P-E, GM bypass, C1–C7).
**Reference (772 mechanics):** `reference/cipsoft-772/tibia-game-master/src/` — `operate.cc`,
`cract.cc`, `crnonpl.cc`, `crmain.cc`, `crplayer.cc`, `info.cc`, `objects.hh`,
`runtime/dat/objects.srv`.
**Reference (772 wire):** not used (no packet changes).

---

## 0. Where chain-push actually lives in 772

There is **no** chain-push in `::Move` / `CheckMapDestination`. It comes from **Gate B** —
the per-creature `MovePossible(Execute=true)` predicate that `CheckMapDestination` invokes on
the **moving** creature (`operate.cc:515-516`).

For a **monster** on the same floor, that dispatches to `TMonster::MovePossible`
(`crnonpl.cc:2141-2293`), whose tail is the 100-attempt **kick-and-retry loop**
(`crnonpl.cc:2185-2288`). Each iteration scans the destination tile; for a creature blocker
it calls `TMonster::KickCreature` (`crnonpl.cc:3036-3098`), which tries the four N/S/W/E
offsets and — at `crnonpl.cc:3066` — calls the **blocker's own**
`MovePossible(Dest, Execute=true)` to validate the escape tile. That recursive call runs the
same kick loop, so a third creature on the escape tile gets shoved in turn. This is the
"X - X - X" chain the user remembered.

Call chain (772):

```
player push packet
  → receiving.cc:233 CMoveObject
  → cract.cc:1123 TCreature::ToDoMove   (1000 ms + walk cooldown for creature-container)
  → cract.cc:823    Execute TDMove
  → cract.cc:475    TCreature::Move
  → operate.cc:1282 ::Move
      ├─ operate.cc:1356 CheckTopMoveObject
      ├─ operate.cc:1358 CheckMoveObject            (Gate A — race unpushable)
      └─ operate.cc:1359 CheckMapDestination        (Gate C + Gate B)
           └─ operate.cc:516 MovingCreature->MovePossible(Execute=true)
                └─ crnonpl.cc:2141 TMonster::MovePossible
                     └─ crnonpl.cc:2185 for Attempt 0..100
                          └─ crnonpl.cc:2241 KickCreature
                               └─ crnonpl.cc:3066 Creature->MovePossible(Execute=true)  ← recursion (chain)
```

### Conditions for the chain to fire (772)

All of the following must hold for the moving creature (the one being pushed):

1. It is a **monster** (`Type == MONSTER`). Players and NPCs have no kick loop.
2. `OrigZ == DestZ` **or** `Type != MONSTER` (`operate.cc:515`) — i.e. same-floor push for a
   monster. Cross-floor monster pushes **skip** `MovePossible` entirely (C5 in the prior
   audit) and therefore skip the chain.
3. `State ∈ {ATTACKING, PANIC}` (`crnonpl.cc:2194`) — otherwise a creature on the dest tile
   is a hard `return false`.
4. `Target != 0` (`crnonpl.cc:2198`).
5. `RaceData[Race].KickCreatures` (`crnonpl.cc:2202`).
6. The blocker is not the mover's target or master (`crnonpl.cc:2212`), not an unpushable-race
   creature (`crnonpl.cc:2216`), not invisible (unless `SeeInvisible`, `crnonpl.cc:2221`),
   not an NPC (`crnonpl.cc:2225`), and not a player — a player blocker instead clears
   `Target = 0` and throws `EXHAUSTED` (`crnonpl.cc:2236-2238`).

### What does **not** chain

- **Pushed players and NPCs:** `objects.srv` TypeID 99 (creature container) is
  `Flags = {Container,Unpass}` (`objects.srv:61-64`). So `CoordinateFlag(dest, UNPASS)` is
  true whenever any creature stands on the dest tile, and the base
  `TCreature::MovePossible` (`crmain.cc:883-898`) returns `false` → `NOROOM`
  (`operate.cc:517-518`). No kick loop is entered for a player/NPC moving creature.
- **Cross-floor pushes of monsters:** C5 skips `MovePossible` for `dz != 0` monsters, so the
  dest is checked only by the elevation gate, `ThrowPossible`, and (for non-self pushes)
  `AVOID`/PZ→non-PZ. A creature on the dest tile does **not** block — creatures stack.

---

## 1. Rust parity status for the chain

The chain is **ported**, but only verified from the monster-walk entry point, not from
`player_push_creature`.

| Link in the chain | Rust | Status |
|---|---|---|
| `CheckMapDestination` → `MovePossible(Execute=true)` | `check_push_destination` → `monster_move_possible_push` (`creature/move_possible.rs:165`) | ✓ wired |
| `TMonster::MovePossible` kick loop | `monster_kick_before_step` (`monster_push.rs:115`) | ✓ ported (100-attempt loop) |
| `TMonster::KickCreature` | `monster_kick_creature` / `monster_kick_creature_inner` (`monster_push.rs:401,416`) | ✓ ported |
| Recursive `MovePossible(Execute=true)` on the blocker | `monster_move_possible_execute_for_kick` (`monster_push.rs:557`) | ✓ ported (F2 fix) |
| Cycle guard | `MAX_KICK_DEPTH = 8` (`monster_push.rs:55`) | Rust-only; 772 relies on N/S/W/E order + skip-kicker-tile |
| Tests | `f2_chain_push_three_monsters`, `f2_chain_push_no_stacking`, `f2_chain_push_cycle_guard`, `f2_chain_push_boxed_in_kills` (`monster_push_tests.rs`) | ✓ for `on_walk` entry; ✗ for `player_push_creature` entry |

**Conclusion:** the chain mechanism is correct. The defects below are about the **gates
around** it and the **entry path** from the player push, not the recursion itself.

---

## 2. Findings

| # | Finding | Severity | Outcome differs? |
|---|---|---|---|
| **D1** | `tile_is_bank_and_passable` (Rust `CoordinateFlag(BANK) && !CoordinateFlag(UNPASS)`) only scans items + the `BLOCKSOLID` tile flag; **creatures are never UNPASS** in the Rust model. 772 TypeID 99 = `{Container,Unpass}`, so any creature on the tile makes `CoordinateFlag(UNPASS)` true. Net: a player/NPC Gate B wrongly *passes* on an occupied tile; the block lands later in `tile_query_add_creature` as `NotPossible` instead of 772 `NOROOM`. <ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/monster_push.rs" lines="362-386" /> **Fixed** — `tile_is_bank_and_passable` now returns `false` when `body.creatures` is non-empty. Test: `player_push_player_onto_occupied_tile_is_noroom`. | **High** | ~~Yes~~ Fixed |
| **D2** | **Cross-floor push onto an occupied tile.** 772: `jump=true` → `JumpPossible(dest,false)` rejects only `UNPASS && UNMOVE`; a creature is `Unpass` but not `Unmove` → **push succeeds, creatures stack**. For a *monster* cross-floor, `MovePossible` is skipped entirely (C5). Rust matches both gate results — then `tile_query_add_creature` (`game_world_player_throw.rs:202`) blocks on `body.creatures` → `NotPossible`. **Fixed** — `tile_query_add_creature` removed from the push path (D3); cross-floor stacking now matches 772. Test: `player_push_across_floor_onto_occupied_tile_stacks`. | **High** | ~~Yes~~ Fixed |
| **D3** | `tile_query_add_creature` has **no counterpart in the 772 push path** — `operate.cc:1356-1359` runs only `CheckTopMoveObject` / `CheckMoveObject` / `CheckMapDestination`. It is currently load-bearing only because of D1. Extra gates it adds on the push path: ghost-mode, `NOFIELDBLOCKPATH`, `BLOCKSOLID`→`NotEnoughRoom`, and (for monsters) `FLOORCHANGE\|TELEPORT`. <ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/game_world_player_throw.rs" lines="193-205" /> **Fixed** — `tile_query_add_creature` call removed from `player_push_creature`; creature blocking now lives in Gate B via D1. | **High** | ~~Yes~~ Fixed |
| **D4** | **Stale origin.** 772 `CheckMapDestination` reads `Orig*` from `GetObjectCoordinates(Obj)` = the creature's **live** position (`operate.cc:482`). Rust uses the packet-time `from_pos` for the 1-tile range cap, the elevation gate, PZ→non-PZ **and** `throw_possible`. With P-B's 1000 ms `ToDoMove` wait the target routinely moves in between; only the `object_in_range` re-check uses live pos. <ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/game_world_player_throw.rs" lines="264-271" /> **Fixed** — `check_push_destination` now passes `target_pos` (live) as `from`. Test: `player_push_uses_live_target_position`. | **High** | ~~Yes~~ Fixed |
| **D5** | Hard-block ordering in the kick loop. 772 `MovePossible` walks the tile stack and `return false` **immediately** on a creature hard block (target/master/NPC/unpushable/invisible), so `KickBoxes` is never reached. Rust `break`s the creature loop and then still runs `monster_kick_boxes`. <ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/monster_push.rs" lines="180-233" /> | Medium | Yes — boxes get shoved/destroyed on a push that then fails |
| **D6** | `monster_move_possible_planning` rejects `FLOORCHANGE\|TELEPORT` tiles. `TMonster::MovePossible` (`crnonpl.cc:2141-2293`) has no such check. <ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/monster_ai.rs" lines="1818-1826" /> | Medium | Yes — cannot push a monster onto stairs/teleport; 772 can |
| **D7** | Kick-loop target gate is `attack_target.is_some() \|\| follow_target.is_some()`; 772 checks the single `this->Target` (`crnonpl.cc:2198`). Follow-only monsters chain-push in Rust but not in 772. | Low | Yes (edge) |
| **D8** | No test drives the chain from `player_push_creature` — the exact scenario the user described is unverified end-to-end. **Fixed** — `player_push_chains_three_monsters` + `player_push_monster_without_target_is_noroom` added. | Medium | ~~Unknown~~ Verified |

---

## 3. Proposed changes

D1 touches a helper shared with `KickBoxes` / `ClearField`, and D2/D3 *remove* a safety gate,
so they need explicit sign-off before landing.

### D1 — creatures are UNPASS in 772

```diff
--- a/crates/tfs-rust-core/src/monster_push.rs
+++ b/crates/tfs-rust-core/src/monster_push.rs
@@ fn tile_is_bank_and_passable
+        // 772 `objects.srv` TypeID 99 (creature container) = `{Container,Unpass}`
+        // (`objects.srv:61-64`), so any creature on the tile makes
+        // `CoordinateFlag(UNPASS)` true.
+        if !body.creatures.is_empty() {
+            return false;
+        }
```

### D2 + D3 + D4 — drop `tile_query_add_creature` on the push path; use live origin

```diff
--- a/crates/tfs-rust-core/src/game_world_player_throw.rs
+++ b/crates/tfs-rust-core/src/game_world_player_throw.rs
@@ fn player_push_creature
-        self.check_push_destination(
-            moving_creature,
-            from_pos,
-            to_pos,
-            target_pos,
-            moving_is_monster,
-            can_push_all,
-        )?;
+        // D4: 772 `CheckMapDestination` reads `Orig*` from `GetObjectCoordinates(Obj)` —
+        // the creature's live position, not the packet origin (`operate.cc:482`).
+        self.check_push_destination(
+            moving_creature,
+            target_pos,
+            to_pos,
+            target_pos,
+            moving_is_monster,
+            can_push_all,
+        )?;
-        let Some(to_tile) = self.map.get_tile(to_pos) else {
-            return Err(ReturnValue::NotPossible);
-        };
-        let query_flags = if can_push_all {
-            crate::walk::FLAG_NOLIMIT
-        } else {
-            0
-        };
-        let rv = crate::walk::tile_query_add_creature(self, to_tile, moving_creature, query_flags);
-        if rv != ReturnValue::NoError {
-            return Err(rv);
-        }
+        // D3: 772 `::Move` (`operate.cc:1356-1359`) runs no TFS-style `queryAdd` on the
+        // push path — Gate A/B/C are the whole gate. Creature blocking now lives in
+        // `tile_is_bank_and_passable` (D1), so cross-floor stacking matches 772 (D2).
```

### D5 — hard block aborts before `KickBoxes`

```diff
--- a/crates/tfs-rust-core/src/monster_push.rs
+++ b/crates/tfs-rust-core/src/monster_push.rs
@@ fn monster_kick_before_step
-        // Boxes / hazard fields — `MovePossible` `UNPASS`/`AVOID` branches
-        if can_kick_boxes {
-            self.monster_kick_boxes(mover, dest, state);
-        }
+        // D5: 772 `MovePossible` `return false`s on a creature hard block before the
+        // item branch is reached (`crnonpl.cc:2194-2233`). Only run `KickBoxes` when no
+        // creature hard block was hit. Track via a flag set on each `break` above.
```

### D6 — drop `FLOORCHANGE|TELEPORT` from the push planning path

Either narrow `monster_move_possible_planning`'s flag set when called from
`monster_move_possible_push`, or split a push-specific planner. The walk-time planner keeps
the current flags.

### D7 — kick target gate

```diff
-        let has_target = target_attack.is_some() || target_follow.is_some();
+        // 772 checks the single `this->Target` (`crnonpl.cc:2198`).
+        let has_target = target_attack.is_some();
```

---

## 4. Verification

```
rtk cargo check -p tfs-rust-core
rtk cargo clippy -p tfs-rust-core -- -D warnings
rtk cargo test -p tfs-rust-core monster_push
rtk cargo test -p tfs-rust-core push
```

## 5. Tests to add

1. `player_push_chains_three_monsters` — actor pushes attacking cyclops (Target set,
   `canPushCreatures`) onto a tile held by a pushable monster which is itself boxed in by a
   third → verify chain via `player_push_creature`, not `on_walk`. (Covers D8.)
2. `player_push_monster_without_target_is_noroom` — same layout, `Target = 0` →
   `NotEnoughRoom`, no kick side effects.
3. `player_push_player_onto_occupied_tile_is_noroom` — asserts D1's error code.
4. `player_push_across_floor_onto_occupied_tile_stacks` — D2 (772 quirk).
5. `player_push_uses_live_target_position` — target walks during the 1000 ms wait; assert
   range cap uses live pos (D4).
6. `player_push_hard_block_does_not_kick_boxes` — D5.

## 6. Suggested landing order

1. ~~**D4 + D8**~~ — ✅ landed (live origin + end-to-end chain-push test from `player_push_creature`).
2. ~~**D1 + D2 + D3**~~ — ✅ landed (creatures are UNPASS in `tile_is_bank_and_passable`; `tile_query_add_creature` removed from the push path; cross-floor stacking matches 772).
3. **D5 + D6 + D7** — smaller behavioral fixes; can be one commit.
