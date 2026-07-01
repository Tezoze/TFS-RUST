# Walk System Audit — Findings & Fixes

**Date:** 2026-07-01
**Auditor:** Devin (GLM-5.2 High)
**Scope:** `crates/tfs-rust-core/src/walk/` (`mod.rs`, `walk_timing.rs`, `walk_tile.rs`), `idle_stimulus.rs`, `monster_ai.rs`, `creature_think.rs`, `creature_todo.rs`
**Reference:** TFS 1.4.2 `src/creature.cpp`, `src/monster.cpp`, `src/game.cpp`, `src/tile.cpp`; 772 `gameserver/src/`, `cract.cc`, `crnonpl.cc`

---

## Methodology

Compared the Rust walk implementation against the C++ reference for:
- Step speed/duration/delay calculations (`getStepSpeed`, `getStepDuration`, `getWalkDelay`, `getEventStepTicks`)
- Walk scheduling (`addEventWalk`, `onWalk`, `checkCreatureWalk`, `stopEventWalk`)
- Step execution (`getNextStep`, `internalMoveCreature`, `onCreatureMove`)
- Monster pathing flow (`Monster::getNextStep`, `IdleStimulus`, `goToFollowCreature`, `onThink` repath cadence)
- Tile traversal (`Tile::queryAdd`, `queryDestination`, floor changes)

---

## Findings

### BUG 1 (HIGH): `on_walk` bypasses `Monster::getNextStep` idle/dead check when `walk_queue` is non-empty

**File:** `crates/tfs-rust-core/src/walk/mod.rs` lines 1217–1237

**C++ behavior:** `Monster::getNextStep` (`src/monster.cpp:1224–1230`) **always** runs its guard checks before popping from the queue:

```cpp
bool Monster::getNextStep(Direction& direction, uint32_t& flags)
{
    if (!walkingToSpawn && (isIdle || getHealth() <= 0)) {
        eventWalk = 0;
        return false;
    }
    // ... then Creature::getNextStep (pop from queue)
```

**Rust behavior:** `on_walk` pops directly from `walk_queue` when it's non-empty, **skipping** the idle/dead guard:

```rust
let pop_dir = if monster {
    if !walk_queue.is_empty() {
        walk_queue.pop_back()          // ← no idle/dead check!
    } else {
        self.monster_next_walk_step(cid, now)  // ← has the check
    }
} else {
    walk_queue.pop_back()
};
```

**Impact:** A monster that becomes idle (no spectators) while it still has queued chase steps will continue walking in Rust. In C++ it stops immediately and sets `eventWalk = 0` (preventing reschedule). This causes unnecessary simulation work and can leave monsters displaced from their spawn when players return.

**Fix:** Route all monster step pops through `monster_next_walk_step` (or at minimum, duplicate the idle/dead guard before the direct pop). The guard is:

```rust
if !walking_to_spawn && (is_idle || health <= 0) {
    // stop walk, do not pop
    return None;
}
```

---

### BUG 2 (HIGH): 1098 monster walk timer re-arms when queue empty — C++ stops and waits for `onThink`

**File:** `crates/tfs-rust-core/src/walk/mod.rs` lines 1392–1422

**C++ behavior:** In `onWalk()` (`src/creature.cpp:215–233`), when `getNextStep` returns false:

```cpp
} else {
    stopEventWalk();           // eventWalk = 0
    if (listWalkDir.empty()) {
        onWalkComplete();
    }
}
// ...
if (eventWalk != 0) {         // eventWalk is 0 → NO reschedule
    eventWalk = 0;
    addEventWalk();
}
```

The walk timer **stops**. The monster's next movement comes from `onThink` (1000ms interval) → `goToFollowCreature` → `startAutoWalk` → `addEventWalk`.

**Rust behavior:** The `else` branch re-arms the timer for 1098 monsters:

```rust
} else if self.monster_should_keep_chase_walk_alive(cid)
    || self.monster_should_keep_dance_walk_alive(cid)
{
    self.schedule_walk_followup_deadline(cid);  // ← re-arms timer!
}
```

**Impact:** 1098 monsters poll every step duration (~400ms) when they can't move, instead of waiting 1 second for `onThink`. This changes the repath cadence fundamentally:
- **C++:** `onThink` (1s) → `goToFollowCreature` → path → `startAutoWalk` → walk timer
- **Rust:** walk timer (~400ms) → `monster_next_walk_step` (no path computation) → re-arm

The Rust `monster_next_walk_step` for 1098 includes dance steps and random steps but **does not repath** — it only pops from the queue or picks a dance/random step. The actual repath happens in `creature_think.rs` via `go_to_follow_creature` on the `FOLLOW_PATH_UPDATE_INTERVAL_MS` (200ms) cadence. So the walk timer re-arm creates a polling loop that doesn't repath, while the think interval handles repathing separately. This dual-path can cause monsters to dance/random-step without a fresh path, leading to erratic movement.

**Fix:** For 1098 monsters, when `getNextStep` returns false and the queue is empty, **stop the walk timer** (`stopped_without_reschedule = true`) and let `onThink` drive the next movement cycle, matching C++. Remove the `monster_should_keep_chase_walk_alive` / `monster_should_keep_dance_walk_alive` re-arm path for the non-beat-driven loop.

---

### BUG 3 (MEDIUM): `on_walk` failed-step `forceUpdateFollowPath` not set for 772 monsters on blocked move

**File:** `crates/tfs-rust-core/src/walk/mod.rs` lines 1276–1318

**C++ behavior:** In `onWalk()` (`src/creature.cpp:207–214`):

```cpp
if (ret != RETURNVALUE_NOERROR) {
    // ...
    forceUpdateFollowPath = true;   // ← always, all creatures
}
```

**Rust behavior:** The 772 monster branch clears the queue and requests idle stimulus, but does **not** set `force_update_follow_path`:

```rust
Err(ret) => {
    // ...
    if self.beat_driven_loop && monster {
        // clears walk_queue, has_follow_path, todo Go actions
        // requests idle_stimulus
        // ← force_update_follow_path NOT set
    } else if let Some(k) = self.creatures.get_mut(cid) {
        if k.base().follow_target.is_some() {
            k.base_mut().force_update_follow_path = true;
        }
    }
}
```

The 772 path clears `has_follow_path` and `force_update_follow_path` is set to `true` via `request_idle_stimulus` → `monster_idle_stimulus` → `monster_idle_prepare_and_enqueue_go` which checks `force_update_follow_path`. But the direct `force_update_follow_path = true` is missing in the error branch itself. The idle stimulus path may compensate, but the explicit signal is lost, which could cause the repath to be skipped if the idle stimulus is deduped or gated.

**Fix:** Set `force_update_follow_path = true` in the 772 monster error branch too, matching C++:

```rust
if self.beat_driven_loop && monster {
    if let Some(k) = self.creatures.get_mut(cid) {
        let base = k.base_mut();
        if base.follow_target.is_some() || base.attack_target.is_some() {
            base.walk_queue.clear();
            base.has_follow_path = false;
            base.force_update_follow_path = true;  // ← ADD THIS
            base.todo.queue.retain(|a| !matches!(a, CreatureAction::Go));
        }
    }
    self.request_idle_stimulus(cid);
}
```

---

### BUG 4 (MEDIUM): `last_step_cost` computed from overall old→new, not last chain segment

**File:** `crates/tfs-rust-core/src/walk/mod.rs` lines 1376–1389

**C++ behavior:** `onCreatureMove` (`src/creature.cpp:485–499`) is called per `Map::moveCreature` in the queryDestination chain. `lastStepCost` is set by the **last** chain segment's from/to:

```cpp
if (from.getPosition().z != to.getPosition().z) {
    creature.lastStepCost = 2;
} else if (from.getPosition().x != to.getPosition().x && from.getPosition().y != to.getPosition().y) {
    creature.lastStepCost = 3;
} else {
    creature.lastStepCost = 1;
}
```

**Rust behavior:** `last_step_cost` is computed from the **overall** `old_pos` (before initial step) to `new_pos` (after all chain steps):

```rust
base.last_step_cost = last_step_cost_for_move(old_pos, new_pos);
```

For most cases this is equivalent (floor changes produce z-diff in both overall and last segment). But for edge cases like a diagonal step onto a floor-change tile where the chain moves cardinally, the overall cost could be 2 (z-change) while the last segment is also 2 (z-change) — same. However, if a diagonal step triggers a chain that ends with a cardinal same-floor move (unlikely but possible with certain tile configs), the costs would differ.

**Impact:** Low in practice — floor change chains almost always end with a z-change. But it's a correctness deviation that could cause subtle timing differences.

**Fix:** Track the last segment's from/to and compute `last_step_cost` from those, matching C++ per-segment semantics:

```rust
// In the Ok(segments) branch:
let last_segment = segments.last();
let (cost_from, cost_to) = match last_segment {
    Some(seg) => (seg.from, seg.to),
    None => (old_pos, new_pos),
};
base.last_step_cost = last_step_cost_for_move(cost_from, cost_to);
```

---

### BUG 5 (LOW): `linear_go_speed_from_profile` applies `player_speed_model` to monsters

**File:** `crates/tfs-rust-core/src/walk/walk_timing.rs` lines 107–118

**Issue:** `linear_go_speed_from_profile` dispatches on `player_speed_model` but is called for **all** creatures (players and monsters) in `walk_timing_speed` and `linear_go_step_duration_ms`:

```rust
fn linear_go_speed_from_profile(go: i32, mech: &crate::formulas::Mechanics) -> i32 {
    match mech.profile.player_speed_model {
        PlayerSpeedModel::BalancedLog => {
            crate::formulas::linear_go_effective_speed(balanced_softened_go(go))
        }
        // ...
    }
}
```

For 772 with `BalancedLog`, monsters with GoStrength > 320 would get log-softened speed, which may not match the 772 reference (`2*go+80` for all creatures). Most monsters have GoStrength < 320 so `balanced_softened_go` returns `go` unchanged, but high-speed monsters (e.g., boss monsters with boosted speed) would be affected.

**Fix:** Add a `WalkSpeedRole` parameter to `linear_go_speed_from_profile` and only apply `player_speed_model` softening for players. Monsters should use `linear_go_effective_speed(go)` directly (the `2*go+80` formula):

```rust
fn linear_go_speed_from_profile(
    go: i32,
    role: WalkSpeedRole,
    mech: &crate::formulas::Mechanics,
) -> i32 {
    match role {
        WalkSpeedRole::MonsterOrNpc => {
            crate::formulas::linear_go_effective_speed(go)
        }
        WalkSpeedRole::Player => match mech.profile.player_speed_model {
            PlayerSpeedModel::EraDefault | PlayerSpeedModel::Classic772 => {
                crate::formulas::linear_go_effective_speed(go)
            }
            PlayerSpeedModel::Retail1098 => tfs_retail_log_speed(go).max(1),
            PlayerSpeedModel::BalancedLog => {
                crate::formulas::linear_go_effective_speed(balanced_softened_go(go))
            }
        },
    }
}
```

---

### BUG 6 (LOW): `schedule_walk_followup_deadline` recomputes delay instead of using 1ms poll like C++

**File:** `crates/tfs-rust-core/src/walk/mod.rs` lines 974–1026

**C++ behavior:** In `addEventWalk` with `ticks == 1` (`src/creature.cpp:317–321`):

```cpp
if (ticks == 1) {
    g_game.checkCreatureWalk(getID());   // sync step
}
eventWalk = g_scheduler.addEvent(createSchedulerTask(ticks, ...));  // ticks still 1
```

The follow-up is a **1ms timer**, which fires `checkCreatureWalk` → `onWalk` → `addEventWalk()` (recomputing the real delay).

**Rust behavior:** `schedule_walk_followup_deadline` (called when `first_step && ticks == 1`) recomputes `getEventStepTicks(false)` and schedules the **full step duration** directly, skipping the 1ms intermediate poll.

The comment says this is intentional ("Recomputing after `last_step` is set tightens rhythm"). The end result is similar (next step fires after step duration), but the C++ 1ms poll provides a synchronization point where `getWalkDelay` is re-evaluated. If conditions change between the sync step and the next timer (e.g., speed condition expires), the Rust path would miss that re-evaluation point.

**Impact:** Minimal in practice — the 1ms poll is an artifact of C++ scheduler mechanics. The Rust optimization is reasonable. Document as an accepted deviation.

---

## Priority & Fix Order

| Bug | Severity | Impact on Monster Pathing | Fix Effort |
|-----|----------|---------------------------|------------|
| **2** | HIGH | Direct — 1098 monsters poll without repathing | Small (remove re-arm, set `stopped_without_reschedule`) |
| **1** | HIGH | Indirect — monsters walk when they should stop | Small (add guard before direct pop) |
| **3** | MEDIUM | 772 monsters may not repath after blocked step | Trivial (add one line) |
| **4** | MEDIUM | Subtle timing differences on floor-change chains | Small (use last segment) |
| **5** | LOW | High-speed monsters get wrong speed on BalancedLog | Small (add role param) |
| **6** | LOW | Accepted deviation — no fix needed | N/A |

**Recommended fix order:** 2 → 1 → 3 → 4 → 5

---

## Verification

After each fix, run:

```bash
rtk cargo check
rtk cargo clippy
rtk cargo test -p tfs-rust-core
```

Specifically watch for:
- `step_speed_tests` in `walk/mod.rs` (lines 1802+)
- Monster chase/dance tests in `idle_stimulus.rs`
- Walk timing tests in `walk_timing.rs`

For behavioral verification, test with:
- 1098 monster chasing a player that kites out of range (Bug 2)
- Monster becoming idle mid-chase (Bug 1)
- 772 monster blocked by a wall (Bug 3)
- Floor change walking (Bug 4)
- High-speed monster (GoStrength > 320) on 772 BalancedLog (Bug 5)
