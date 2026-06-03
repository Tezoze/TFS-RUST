# 772 Monster AI Gap Audit (Targeting + Pathing)

## Scope
- **Rust under review:** `crates/tfs-rust-core/src/monster_ai.rs`, `monster_distance_step.rs`, `pathfinding.rs`, `creature/monster.rs`
- **TVP reference (gameserver):** `gameserver/src/monster.cpp`, `gameserver/src/monsters.cpp`
- **CipSoft-style reference:** `tibia-game-master/src/crnonpl.cc`
- Focus is only 772 monster **target acquisition/retargeting** and **movement/pathing/chase-flee behavior**.

## Executive Snapshot
- Rust currently mixes TFS/TVP-like mechanics with some CipSoft-style tunables, but it is **not yet full parity** with either reference for 772 targeting/pathing.
- The largest behavior gaps are:
  - Missing full strategy model (`nearest/weakest/most-damage/random` weighted selection)
  - Missing CipSoft `LoseTarget` semantics
  - Missing CipSoft hard distance-fighting behavior around fixed 4-tile standoff logic
  - Missing CipSoft house-zone exclusion in target validity checks

## What Rust Does Today
- Target list maintenance:
  - Keeps `opponent_ids`/`friend_ids`, prunes via visibility, updates on appear/move/leave.
  - Functions: `monster_update_target_list`, `monster_on_creature_found`, `monster_on_creature_move`, `monster_prune_creature_lists`.
- Target selection:
  - Modes implemented: `Default`, `Random`, `Nearest`, `AttackRange`, `HealthLow`.
  - Uses `monster_search_target` and `monster_select_target`.
  - Retarget cadence is driven by `change_target_speed` + `change_target_chance` in `monster_on_think_target`.
- Pathing/chase/flee:
  - Uses direct distance-step logic (`get_distance_step`) first, then A* fallback (`get_path_matching`).
  - Includes chase/flee dance logic, follow repath on target move, and return-to-spawn behavior.
  - Uses profile-driven cost model (`Fixed` or `TerrainWeighted`) and profile-driven target distance interpretation.

## Reference Behavior Highlights

### TVP `gameserver/src/monster.cpp`
- Targeting:
  - Uses weighted strategy buckets from monster XML (`nearest`, `weakest`, `mostDamage`, `random`) loaded in `gameserver/src/monsters.cpp`.
  - Weakest bucket uses **max health** in this codepath.
  - Retarget can drop current target via `changeTargetChance`.
- Pathing:
  - Uses `targetDistance` per monster for ranged keep-distance.
  - Uses `getFlightStep`, short dance/random lateral movement at desired distance, and `getPathTo(..., 12)`.

### CipSoft-style `tibia-game-master/src/crnonpl.cc`
- Targeting:
  - Strategy roll via race `Strategy[]` buckets: nearest / lowest current health / most damage / random.
  - Explicit `LoseTarget` chance gate per think cycle.
  - Filters include protection zone, house, invisibility checks, range (10 tiles), z-level.
- Pathing:
  - Distance-fighting semantics are explicit:
    - If throw not possible or non-distance fighter -> close combat chase.
    - If distance fighter + throw possible -> hold near 4-tile distance (step away if <4, step closer if >4, dance at ==4).
  - Uses `SearchFlightField`, `ToDoGo`, and chase/attack scheduling in its own queue model.

## Gap List (Rust vs TVP/CipSoft)

## P0 — High Behavior Mismatch
- **No full weighted target-strategy system in Rust**
  - Rust currently does not implement full weighted `nearest/weakest/most-damage/random` strategy composition from monster data.
  - Rust has `HealthLow` and `Nearest`, but no implemented `most-damage` strategy path and no weighted strategy roll equivalent.
  - Impact: wrong target picks under multi-player pressure compared to both references.

- **No CipSoft `LoseTarget` chance model**
  - Cip reference has explicit `LoseTarget` random drop on current target each cycle.
  - Rust retarget model is based on `change_target_speed/chance` and does not mirror Cip `LoseTarget` behavior.
  - Impact: target stickiness differs (especially PvE kiting/distraction behavior).

## P1 — Medium Behavior Mismatch
- **Distance-fighting model diverges from CipSoft 4-tile regime**
  - Cip logic strongly centers around `DistanceFighting` and a 4-tile keep band with specific branch behavior.
  - Rust uses `target_distance` + generic `get_distance_step`/A* logic; this is closer to TFS-like generalized behavior.
  - Impact: kiting feel and ranged monster spacing cadence can differ.

- **Target validation misses house-zone exclusion used by Cip reference**
  - Cip code excludes targets in protection zone **and house**.
  - Rust `monster_is_target` checks protection zone but not explicit house exclusion in the same way.
  - Impact: edge-case targeting around house boundaries differs.

- **Retarget cadence semantics differ**
  - TVP/Cip implementations have simpler per-stimulus random drop/selection flow.
  - Rust uses tick/cooldown accumulation and phased retarget logic.
  - Impact: when target switches occur can drift from reference timing.

## P2 — Lower Severity / Architectural Differences
- **Scheduling model differs**
  - References are queue-driven (`addWalkToDo`, `ToDoStart`) with legacy wakeup semantics.
  - Rust uses game-loop driven follow/path queue + repath hooks.
  - Impact: usually equivalent outcomes, but frame-to-frame micro-timing may differ.

- **Cip target pool boundaries differ**
  - Cip scans through creature search structures and applies additional filtering details (player-controlled monsters, floor visibility nuances).
  - Rust spectator/visibility scan is cleaner and modernized but not a byte-for-byte behavior clone.

## File-Level Evidence Map
- Rust:
  - `crates/tfs-rust-core/src/monster_ai.rs`
  - `crates/tfs-rust-core/src/monster_distance_step.rs`
  - `crates/tfs-rust-core/src/pathfinding.rs`
  - `crates/tfs-rust-core/src/creature/monster.rs`
- TVP:
  - `gameserver/src/monster.cpp`
  - `gameserver/src/monsters.cpp`
- Cip:
  - `tibia-game-master/src/crnonpl.cc`

## Recommended Fix Order
1. Implement weighted strategy selector parity (`nearest`, `weakest`, `most-damage`, `random`) from loaded monster config.
2. Add Cip-style `LoseTarget` support behind mechanics/profile switch for 772.
3. Add optional strict Cip distance-fighting branch (4-tile distance-fighter model) for 772 profile.
4. Add house-zone target exclusion parity toggle for 772.
5. Add parity tests with deterministic RNG seeds for:
   - strategy selection,
   - lose-target behavior,
   - 4-tile ranged keep-distance transitions.

