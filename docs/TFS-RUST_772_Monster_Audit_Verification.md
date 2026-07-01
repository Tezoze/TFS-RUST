# TFS-RUST 772 — Monster Audit Verification & Gap Report

**Date:** 2026-07-01
**Purpose:** Independent re-verification of the two open monster audits against the *current* `crates/tfs-rust-core` source and the 772 decompile. Records which findings are still accurate, which are stale, and adds newly-discovered gaps.
**Audits verified:**
- [`TFS-RUST_772_Monster_Collision_Bump_Pause_Audit.md`](TFS-RUST_772_Monster_Collision_Bump_Pause_Audit.md) — findings F1–F3
- [`TFS-RUST_772_Monster_AI_Transition_Audit.md`](TFS-RUST_772_Monster_AI_Transition_Audit.md) — findings RC1, RC2, B3, B4, M5

**Reference:** `reference/cipsoft-772/tibia-game-master/src/` (decompile outcomes only)

---

## 1. Executive Summary

Both audits are **substantially correct**, with one important exception: the collision audit's **F1 is now stale** — the code it describes was superseded on 2026-07-01 by the "walk-engine unification Phase 1.3" commit (`c7b4df4`), which already applies most of F1's recommended fix. The remaining findings verify as stated.

| Audit | Finding | Audit status | **Verified status** | Notes |
|-------|---------|--------------|---------------------|-------|
| Collision | F1 — hard-block `Err` path stalls | Open (Critical) | **STALE / effectively resolved** | Code no longer does `retain(!Go)`; it full-clears + yields. See §2.1 |
| Collision | F2 — `KickCreature` planning gate, no chain-push | Open (High) | **VALID / OPEN** | Confirmed unchanged. See §2.2 |
| Collision | F3 — `monster_exhausted_wait_772` clears target unconditionally | Open (Medium) | **VALID / OPEN** | Confirmed unchanged; doc comment cites wrong catch block. See §2.3 |
| Transition | RC1 — `process_creatures_772` ran AI think | Fixed | **CONFIRMED FIXED** | Regen/death-safety only + 3 tests present. See §3.1 |
| Transition | RC2 — missing trailing `ToDoWait(1000)` | Fixed | **CONFIRMED FIXED** | `already_armed` tail present + tests. See §3.2 |
| Transition | B3 — inverted `is_summon` sleep-wake | Open (High) | **VALID / OPEN** | Confirmed inverted polarity. See §3.3 |
| Transition | B4 — sleep from think sweep | Resolved by RC1 | **CONFIRMED RESOLVED** | Consistent with RC1. See §3.4 |
| Transition | M5 — `creature_can_see` Z-split | Open (Low) | **VALID / OPEN** | Confirmed; C++ `IsVisible` re-verified. See §3.5 |

**New gaps discovered (this pass):**
- **N1** — Collision audit F1 section must be rewritten; it documents superseded code and understates that its own §6.2 fix is ~90% landed.
- **N2** — The blocked-step `Err` arm was widened from monsters to *all* todo-execute creatures (players included) and dropped its target gate; residual robustness note on routing through `request_idle_stimulus` guards vs a direct yield.
- **N3** — F2 + F3 compound with the RC1 fix: since RC1 removed think-based target re-acquisition, an F3 target-drop on a spurious F2 kill now has no 1 Hz safety net — the monster is fully dependent on `IdleStimulus` re-acquire after the 1 s wait. Raises the practical severity of F2/F3 above the audit's ratings.

---

## 2. Collision / Bump-Pause Audit — verification detail

### 2.1 F1 — STALE (superseded 2026-07-01)

**Audit claim:** the `Err` arm does `todo.queue.retain(|a| !matches!(a, CreatureAction::Go))` (keeps `Attack`), then `request_idle_stimulus`, which bails on the non-empty queue and defers the attack by `max(attack_delay, 200)` ms → 200 ms – 2000 ms+ stall.

**Current code (`walk/mod.rs` ~1354–1371):**

```rust
if self.creature_uses_todo_execute(cid) {
    if let Some(k) = self.creatures.get_mut(cid) {
        let base = k.base_mut();
        base.walk_queue.clear();
        base.has_follow_path = false;
        base.force_update_follow_path = true;
        base.todo.queue.clear();     // ← FULL clear (was: retain !Go)
        base.todo.locked = false;    // ← unlock
    }
    self.request_idle_stimulus(cid);
}
```

The `retain(!Go)` is gone — replaced by a full `todo.queue.clear()` and `locked = false`. This is F1's recommended `ToDoClear`. Introduced by commit `c7b4df4` ("772 walk-engine unification Phase 0 + 1.1-1.3", 2026-07-01), **after** the audit's 2026-06-28 date.

**Does the residual stall survive?** No, in the normal case. Traced end-to-end:

1. `process_creature_todo` does `next_wakeup.take()` at entry (`walk/mod.rs` ~388), so `next_wakeup == None` when the `Err` arm runs.
2. `request_idle_stimulus` guards (`idle_stimulus.rs` ~148–186):
   - `beat_driven_loop` ✓ · `creature_uses_todo_execute` ✓
   - `walk_timer_idle` → `next_wakeup.is_none()` → **true** (taken in step 1)
   - `creature_todo_queue_empty` → **true** (just cleared) — this is the guard the audit said bailed; it now passes
   - `idle_stimulus_last_ms == server_ms` dedup → passes (a chase `Go` executes on a later beat than the `IdleStimulus` that enqueued it, because `ToDoStart` clamps to `server_ms + 1` → next beat)
   - `has_wait()` → false (cleared)
3. → `creature_todo_yield` (bails only on `locked`, which was cleared) → arms wakeup at `server_ms + 1`.

So the current path is functionally equivalent to the audit's proposed direct `creature_todo_yield`: a hard-blocked monster re-runs `IdleStimulus` on the next beat (~1 ms logical). **F1's stall no longer occurs.**

**Action:** rewrite the F1 section as "resolved/superseded" and update the priority table. See N1, N2 for residual notes.

<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/walk/mod.rs" lines="1352-1372" />
<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/idle_stimulus.rs" lines="146-186" />

---

### 2.2 F2 — VALID / OPEN (no chain-push)

**Confirmed unchanged.** `monster_kick_creature_772` (`monster_push.rs` ~394–410) still validates the blocker's escape tile with the **planning** gate:

```rust
let can_occupy = match self.creatures.get(blocker) {
    Some(CreatureKind::Monster(_)) => {
        self.monster_move_possible_planning_772(blocker, try_pos)  // Execute=false
    }
    _ => false,
};
if !can_occupy { continue; }
self.move_creature_on_map(blocker, blocker_pos, try_pos);          // forced relocate
```

C++ `KickCreature` calls `Creature->MovePossible(Dest, Execute=true)` (`crnonpl.cc:3066`), which recursively kicks whatever creature sits on the escape tile (chain-push). The Rust planning gate treats a pushable monster on the escape tile as passable (`continue`) and then forcibly relocates the blocker *onto* it — producing the stacking + spurious-kill behavior the audit describes. **The recursive kick is still missing.** F2 stands as written.

<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/monster_push.rs" lines="394-411" />

---

### 2.3 F3 — VALID / OPEN (unconditional target clear; wrong catch cited)

**Confirmed unchanged.** `monster_exhausted_wait_772` (`idle_stimulus.rs` ~195–208) still calls `base.clear_targets()` unconditionally, and there is still a single `MonsterKickOutcome::Exhausted` variant used for **both** throw sites:

- player-tile: `Some(CreatureKind::Player(_)) => return MonsterKickOutcome::Exhausted` (`monster_push.rs` ~185)
- kick-kill: `if !self.monster_kick_creature_772(...) { return MonsterKickOutcome::Exhausted; }` (`monster_push.rs` ~191)

Both flow to `monster_exhausted_wait_772(cid)` → `clear_targets()`. C++ `Execute` catch (`cract.cc:870-888`) does **not** clear `Target`; only the player-tile throw site clears it (`crnonpl.cc:2237`). So on a kick-kill, Rust drops aggro where C++ keeps it. **F3 stands.**

The doc comment on `monster_exhausted_wait_772` still cites `crnonpl.cc:2890-2898` (the `IdleStimulus` catch) — the wrong catch block, as the audit notes. This mis-citation is also confirmed present.

<ref_snippet file="/mnt/storage2/TFS_RUST/crates/tfs-rust-core/src/idle_stimulus.rs" lines="188-209" />

---

### 2.4 "What Already Matches" (audit §5)

Spot-checked (not exhaustively re-verified): kicker gate, hard-block identification, `KICK_DIRS_772` fixed order, full-HP kill damage attribution, and the `for _attempt in 0..100` kick-and-retry loop (`monster_push.rs` ~138–198) all match the audit's claims. No divergence found in the rows checked.

---

## 3. AI Transition Audit — verification detail

### 3.1 RC1 — CONFIRMED FIXED

`process_creatures_772` (`creature_think.rs` ~119–134) is now regen/death-safety only — it iterates all creatures and applies `apply_creature_death` when `hp <= 0`. No `monster_on_think`, no target clear, no idle-status update. The three regression tests exist: `process_creatures_772_does_not_call_on_think`, `process_creatures_772_applies_death_safety`, `process_creatures_772_does_not_clear_targets`.

### 3.2 RC2 — CONFIRMED FIXED

`monster_idle_stimulus_inner` (`idle_stimulus.rs` ~1231–1239) has the unconditional trailing fallback with the `already_armed` guard (`!todo.is_empty() || next_wakeup.is_some()`) → `idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS)`. Test `rc2_idle_stimulus_does_not_double_schedule_when_already_armed` present.

### 3.3 B3 — VALID / OPEN

`monster_sleep_wake_on_creature_move` (`idle_stimulus.rs` ~355–362) still has the inverted polarity:

```rust
CreatureKind::Monster(m) => {
    !m.base.is_summon() && m.opponent_ids.is_empty() && !m.is_hostile
}
```

Wakes for wild monsters (`!is_summon()`), stays asleep for player summons — the opposite of C++ `CreatureMoveStimulus` (`crnonpl.cc:2955-2958`). B3 stands as a one-line fix.

### 3.4 B4 — CONFIRMED RESOLVED BY RC1

`monster_update_idle_status` is no longer reachable from the 772 think sweep (RC1 removed `monster_on_think` from `process_creatures_772`). Consistent with the audit.

### 3.5 M5 — VALID / OPEN (C++ reference re-verified)

`creature_can_see` (`game_world_spectators.rs` ~59–70) has the extra underground block:

```rust
} else if my_z >= 8 {
    if tz < 8 { return false; }         // ← 772 IsVisible does NOT have this
    if (my_z - tz).abs() > 2 { return false; }
}
```

Re-verified against the decompile — `TConnection::IsVisible` (`connections.cc:357-378`):

```cpp
if(PlayerZ <= 7){
    if(z > 7){ return false; }
}else{
    if(std::abs(PlayerZ - z) > 2){ return false; }   // underground CAN see surface
}
```

C++ 772 has no `z < 8` rejection for underground viewers. M5 stands. Caveat (as the audit notes): `IsVisible` is the client-viewport function; the Rust `creature_can_see` is used for AI target selection and mirrors TFS `canSee` semantics, so this is a 1098-vs-772 divergence, low impact (multi-floor ramp edge case).

<ref_snippet file="/mnt/storage2/TFS_RUST/reference/cipsoft-772/tibia-game-master/src/connections.cc" lines="357-378" />

---

## 4. New Findings / Gaps

### 4.1 N1 — Collision audit F1 documents superseded code (Doc bug, must fix)

**Severity:** Documentation (High — misleads future work)

The F1 section (audit §4.1) describes the `retain(!Go)` + `defer_attack_after_go` stall path, which no longer exists after commit `c7b4df4`. Its §6.2 fix ("full `ToDoClear` + direct `creature_todo_yield`") is ~90% already implemented — the queue is now fully cleared and a yield is reached. Leaving F1 open at **Critical** will send an implementer to fix an already-fixed bug.

**Recommended action:**
- Reclassify F1 as **Resolved (superseded by walk-engine unification Phase 1.3)**.
- Note the one residual deviation from the §6.2 recommendation (N2 below).
- Update the audit's priority table (§6.1) and cross-references.

### 4.2 N2 — Blocked-step `Err` arm widened to all creatures; routes through `request_idle_stimulus` guards

**Severity:** Low (robustness / parity nuance)

Two behavioral changes rode in with the Phase 1.3 rewrite that the audit never covered:

1. **Widened population.** The arm is now gated by `creature_uses_todo_execute(cid)` (monsters **and** players), not `beat_driven_loop && is_monster`. And the previous `if follow_target.is_some() || attack_target.is_some()` gate was dropped — the clear is now unconditional. For monsters this is *more* faithful to C++ (`ToDoClear` is unconditional, `cract.cc:871`). For players it means a blocked walk full-clears the ToDo queue and yields into `player_idle_stimulus`, which re-arms only if an attack target exists (otherwise the player simply stops). This appears intended (matches `crplayer.cc:388-405`) but is untested at this seam — worth a targeted test.

2. **Indirect yield.** The audit's §6.2 recommends a **direct** `creature_todo_yield` to bypass `request_idle_stimulus`'s guards. The landed code calls `request_idle_stimulus` instead. In the traced hard-block scenario all guards pass (see §2.1), but the extra guards (`walk_timer_idle`, `idle_stimulus_last_ms` dedup) add coupling that a direct yield would not. If any future change causes `IdleStimulus` to run and enqueue a *same-beat* action that then blocks, the dedup guard could suppress the re-arm. Low probability given the `+1` clamp, but a direct yield remains the more robust shape.

**Recommended action:** add a test `hard_block_reruns_idle_next_beat` (monster whose only path step lands on its own target) asserting `next_wakeup == server_ms + 1` and queue empty; and a player analogue asserting the player stops cleanly. Optionally switch to a direct `creature_todo_yield` per the original §6.2 recommendation.

### 4.3 N3 — F2 + F3 severity is amplified by the RC1 fix

**Severity:** Raises practical impact of F2/F3

Before RC1, `process_creatures_772` ran `monster_on_think` every ~1.75 s, which re-validated/re-acquired targets. That 1 Hz sweep partially masked F3's aggro-drop: a monster that dropped its target on a kick-kill could re-acquire on the next think sweep. RC1 (correctly) removed that sweep — AI is now purely `IdleStimulus` / stimulus-driven. Consequently:

- An F2 spurious kick-kill (dense convoy, blocker boxed in) → F3 target drop → the monster now has **no 1 Hz re-acquire**. It must wait the full 1000 ms `EXHAUSTED` window and re-acquire inside `IdleStimulus`. If nothing re-stimulates it, it can idle → sleep (via the correct `IdleStimulus` sleep path).

This is not a new bug — it is the intended post-RC1 architecture — but it means **F2 and F3 are now the sole guardians of chase continuity in dense groups**. Their effective severity is higher than the audit's "High/Medium" ratings suggest. Fixing F2 (chain-push, eliminates most spurious kills) and F3 (preserve target on kick-kill) should be prioritized together.

**Recommended action:** treat F2 + F3 as a single P1 workstream; land F3 (small) first so target preservation is in place before F2's chain-push reduces the kill rate.

---

## 5. Verification Scope & Method

- Read and traced the exact functions cited by both audits in the current source (`walk/mod.rs`, `idle_stimulus.rs`, `monster_push.rs`, `creature_think.rs`, `creature_todo.rs`, `creature/base.rs`, `game_world_spectators.rs`, `monster_ai.rs`).
- Re-verified two C++ citations directly against the decompile: `TConnection::IsVisible` (`connections.cc:357-378`, M5) and the `monster_kick_creature_772` planning-gate substitution vs `crnonpl.cc:3066` (F2).
- Confirmed fix landing via `git log` on `walk/mod.rs` (commit `c7b4df4`, 2026-07-01) for the F1 supersession.
- **Not** exhaustively re-verified: every row of the "What Already Matches" tables in both audits, and the full RC1/RC2 test bodies (existence and shape confirmed, not line-by-line re-run). No `cargo test` was run for this pass.

---

## 6. Recommended Next Actions

| Priority | Item | Effort |
|----------|------|--------|
| P0 | Update collision audit: reclassify **F1** as resolved/superseded (N1); adjust priority table | Trivial (doc) |
| P1 | Fix **F3** (split kick-kill vs player-tile target semantics) — small, unblocks F2 | Small |
| P1 | Fix **F2** (execute-mode recursive chain-push + cycle guard) | Medium |
| P2 | Fix **B3** (correct `is_summon` sleep-wake polarity) | Trivial |
| P3 | Add tests per **N2** (hard-block re-path for monster + player) | Small |
| P3 | Fix **M5** (align AI Z-visibility with 772 `IsVisible`, gated on `beat_driven_loop`) | Small |

All C++-referenced fixes must cite `gameserver`/decompile file + function in the module header per `tfs-cpp-references`.

---

## 7. Implementation Step List

Ordered for minimal churn and safe interleaving. F1 requires **no code** (already resolved by `c7b4df4`). Each step: write failing tests first, implement, then `rtk cargo test -p tfs-rust-core` before moving on. F2/F3 share the `monster_push` + `walk/mod.rs` seam — run the full suite between them.

### Step 0 — Doc reclassification (N1) — P0, trivial ✅
- [x] In `TFS-RUST_772_Monster_Collision_Bump_Pause_Audit.md`, reclassify **F1** as **Resolved (superseded by walk-engine unification Phase 1.3, commit `c7b4df4`, 2026-07-01)**.
- [x] Update the F1 severity/priority rows in §1 and §6.1; add a pointer to this verification doc §2.1.
- [x] Note the one residual deviation from §6.2 (indirect yield via `request_idle_stimulus`) — see Step 5.

### Step 1 — F3: split EXHAUSTED target semantics — P1, small (do first, unblocks F2) ✅
**Files:** `monster_push.rs`, `idle_stimulus.rs`, `walk/mod.rs`
**C++ ref:** `Execute` catch `cract.cc:870-888`; throw sites `crnonpl.cc:2237` (player-tile, clears `Target`) vs `crnonpl.cc:2241-2242` (kick-kill, preserves `Target`).

- [x] Add variant `MonsterKickOutcome::ExhaustedDropTarget` alongside `Exhausted` in `monster_push.rs`.
- [x] Player-tile throw site (`monster_push.rs` ~185): return `ExhaustedDropTarget`.
- [x] Kick-kill throw site (`monster_push.rs` ~191): keep returning `Exhausted` (target preserved).
- [x] Change `monster_exhausted_wait_772(cid)` → `monster_exhausted_wait_772(cid, clear_target: bool)`; only call `base.clear_targets()` when `clear_target`.
- [x] Update the `on_walk` invocation (`walk/mod.rs` ~1327): match both variants — `Exhausted => wait(cid, false)`, `ExhaustedDropTarget => wait(cid, true)`.
- [x] Fix the `monster_exhausted_wait_772` doc comment to cite `cract.cc:870-888` (Execute catch) + the two throw sites — not `crnonpl.cc:2890-2898`.
- [x] Tests: `f3_kick_kill_preserves_target`, `f3_player_tile_clears_target`, `f3_kick_kill_reengages_same_target`.

### Step 2 — F2: recursive chain-push — P1, medium ✅
**Files:** `monster_push.rs`, `monster_ai.rs`
**C++ ref:** `TMonster::KickCreature` `crnonpl.cc:3036-3098`; recursive `Creature->MovePossible(Dest, Execute=true)` `crnonpl.cc:3066`.

- [x] Add `monster_move_possible_execute_for_kick_772(blocker, try_pos, kicker_pos, now)`: run the planning gate first; if it passes but a **pushable monster** sits on `try_pos`, recursively `monster_kick_creature_772` that creature (chain-push) before declaring the tile occupiable. Hard blocks already fail via planning.
- [x] Point `monster_kick_creature_772`'s escape-tile validation (`monster_push.rs` ~396) at the new execute-mode gate instead of `monster_move_possible_planning_772`.
- [x] Add an explicit cycle guard: `MAX_KICK_DEPTH` (e.g. 8) threaded through a `_inner(depth)` variant; C++ relies on offset order + skip-kicker, Rust must be explicit.
- [x] Snapshot blocker IDs before mutating (borrow-checker; mirror the existing collect-then-mutate pattern).
- [x] Tests: `f2_chain_push_three_monsters`, `f2_chain_push_no_stacking`, `f2_chain_push_boxed_in_kills` (regression), `f2_chain_push_cycle_guard`, `f2_dense_convoy_fluid`.
- [x] Run full `tfs-rust-core` suite (F2+F3 share the seam) — 476 passed, 0 failed.

### Step 3 — B3: fix sleep-wake polarity — P2, trivial
**File:** `idle_stimulus.rs` (~355-362)
**C++ ref:** `TMonster::CreatureMoveStimulus` `crnonpl.cc:2955-2958` (wake for players + player-controlled summons, not wild monsters).

- [ ] Replace the monster arm with `m.base.is_summon()` (wake for player summons). Remove the `opponent_ids.is_empty() && !m.is_hostile` band-aid.
- [ ] Verify `is_summon()` maps to C++ `IsPlayerControlled()` (player-owned only; NPC summons should not wake).
- [ ] Tests: `sleep_wake_wakes_for_player_summon`, `sleep_wake_does_not_wake_for_wild_monster`, `sleep_wake_wakes_for_player`.

### Step 4 — M5: Z-visibility alignment — P3, small (edge case)
**File:** `game_world_spectators.rs` (~59-70)
**C++ ref:** `TConnection::IsVisible` `connections.cc:357-378` (underground: only `abs(dz) > 2` rejects).

- [ ] Remove the `if tz < 8 { return false; }` underground-rejection so underground viewers can see surface within 2 floors.
- [ ] If 1098 shares `creature_can_see`, gate the change on `beat_driven_loop` (772-only) or split the function; confirm `protocol_can_see` (client viewport) is untouched.
- [ ] Test: `creature_can_see_underground_to_surface`.

### Step 5 — N2: F1 regression coverage (+ optional hardening) — P3, small
**Files:** `walk/mod.rs` (tests; optional edit)

- [ ] Add `hard_block_reruns_idle_next_beat`: chasing monster whose only path step lands on its own target → assert `next_wakeup == server_ms + 1` and `todo.queue.is_empty()` after the blocked step.
- [ ] Add a player analogue: blocked player walk with no attack target → assert the player stops cleanly (queue empty, no re-arm).
- [ ] Optional: swap the Err-arm `request_idle_stimulus(cid)` for a direct `creature_todo_yield(cid)` per the original F1 §6.2 recommendation, to drop the extra guard coupling. Only if the regression tests pass unchanged.

### Cross-cutting
- [ ] After all steps: `rtk cargo test -p tfs-rust-core` green; `rtk cargo clippy` no new warnings.
- [ ] Append lessons for F2/F3/B3 to `tasks/lessons.md` (per the collision audit §7.3 wording).
- [ ] Every ported fix cites `gameserver`/decompile file + function in the module header (`tfs-cpp-references`).
