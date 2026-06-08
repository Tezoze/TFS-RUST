# Diagnostic Findings: 772 Monster AI Follow Freeze & Walk Delay Lock

We investigated the monster AI targeting, pathing, and walk scheduling logic under `clientVersion = 772` and `1098` compared to the `gameserver/src/` C++ codebase (TVP 7.72) and 772 decompile (`tibia-game-master/src/`) references. 

We identified two high-probability bugs causing monsters to freeze upon reaching their follow target and fail to chase when the target walks away, plus one unrelated test failure.

---

## Bug 1: `has_follow_path` Logic & A* Restricted Search

### Root Cause
1. In `crates/tfs-rust-core/src/monster_ai.rs`, when a follow target moves, the walk queue is cleared but `m.base.has_follow_path` is explicitly set to `true` if it was `false`:
   ```rust
   // monster_on_follow_creature_moved
   if !has_path {
       if let Some(k) = self.creatures.get_mut(monster_id) {
           k.base_mut().has_follow_path = true; // <--- BUG!
       }
   }
   ```
2. When the monster subsequently repaths via `go_to_follow_creature`, it calls `monster_path_search_params` which sets `fpp.full_path_search = !has_follow_path`. Since `has_follow_path` was forced to `true`, `full_path_search` becomes `false`.
3. In `crates/tfs-rust-core/src/pathfinding.rs`, when `full_path_search` is `false`, the search box is restricted to the initial direction offset:
   ```rust
   let dx_max = if offset_x >= 0 { fpp.max_target_dist } else { 0 };
   if (test.x as i32) > (target.x as i32) + dx_max { return false; }
   ```
   If a player runs outside this narrow restricted box (which is likely if they walk away), A* searches fail immediately (`None`).
4. Although a failed path search clears `has_follow_path` (sets it to `false`), the next time the target moves, `monster_on_follow_creature_moved` is called again and immediately forces `has_follow_path` back to `true`, locking the monster into doing failing restricted searches forever.

### Legacy Parity Check
In the `gameserver/src/` C++ codebase, follow repathing is gated by `hasFollowPath` in `onCreatureMove`. If `hasFollowPath` is `false`, it does not repath inside the move event (it waits for the next think loop to perform a full search). More importantly, the C++ codebase never sets `hasFollowPath = true` inside the movement callback.

For 772 (`tibia-game-master/src/crnonpl.cc`), there is no path flag gating on target move events. So monsters should repath, but since they have no path, `has_follow_path` must remain `false` to perform a full search.

---

## Bug 2: Walk Delay Rescheduling Lock

### Root Cause
1. When a monster is updated with a new follow path in `monster_try_apply_chase_path` and `monster_start_follow_step`, the walk timer is cancelled and restarted:
   ```rust
   self.stop_event_walk(cid);
   self.creature_start_chase_auto_walk(cid);
   ```
2. `creature_start_chase_auto_walk` calls `add_event_walk(cid, true, Instant::now())`, which computes step ticks.
3. In `crates/tfs-rust-core/src/walk.rs`, `get_walk_delay` computes the remaining step duration. However, it is not clamped to `>= 0`. When a monster is standing still, `walk_delay` is negative.
4. In `get_event_step_ticks`, the overdue condition is:
   ```rust
   if only_delay && step_duration > 0 && walk_delay == 0 {
       1
   } else {
       step_duration * base.last_step_cost as i64
   }
   ```
   Because `walk_delay` is negative (not exactly `0`), it falls through to the `else` block and returns the full `step_duration` (e.g. 950ms on 772).
5. If the target player keeps moving, the monster's walk timer is cancelled and rescheduled for `now + 950ms` on every move, effectively locking the monster in place forever because it is constantly waiting out the step cooldown.

### Legacy Parity Check
1. In the `gameserver/src/` C++ codebase, walk delay checks are clamped to `>= 0`, meaning a standing-still creature always returns `0` delay.
2. The `gameserver/src/` C++ `startAutoWalk()` only starts the event walk if `eventWalk == 0`. It does NOT cancel and restart an active walk timer.
3. If the walk delay is `<= 0` and `onlyDelay` is `true`, C++ returns `1` ms (execute immediately), even if it was overdue (negative delay).

---

## Unrelated Test Failure

### Root Cause
- Test `shipped_1098_formulas_match_era_defaults` fails because `formulas.rs` defines the default `player_speed_model` for version 1098 as `EraDefault`, whereas `data/formulas/1098.lua` maps `playerSpeed = "retail"` to `Retail1098`.
- Updating the default `player_speed_model` to `Retail1098` in `formulas.rs` for version 1098 resolves this discrepancy.

---

## Proposed Fixes

1. **`monster_on_follow_creature_moved`**:
   - For 1098: If `has_path` is `false`, return early (no-op).
   - For 772: Allow repathing, but do not modify/set `has_follow_path = true`.
2. **Rescheduling**:
   - Remove the `self.stop_event_walk(cid)` calls from `monster_try_apply_chase_path` and `monster_start_follow_step`. Let the active walk timer continue naturally.
3. **Walk Cooldown**:
   - Clamp the delay in `get_walk_delay` to `.max(0)`.
   - Remove `&& walk_delay == 0` from `get_event_step_ticks` so that `walk_delay <= 0` (overdue/standing still) correctly triggers a `1` ms delay walk check when `only_delay` is true.
   - Update the overdue test expectation in `walk.rs` from `300` to `1` to align with the correct C++ behavior.
4. **Speed Model Default**:
   - Change `player_speed_model: PlayerSpeedModel::EraDefault` to `Retail1098` in `formulas.rs` for version 1098.
