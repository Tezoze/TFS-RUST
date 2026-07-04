# Beat-Unify Branch Inventory — Phase 1

**Date:** 2026-07-04
**Status:** 🟨 DRAFT — K rows pending user sign-off.
**Source:** `grep -rn beat_driven_loop crates/tfs-rust-core/src` → **204 matches / 27 files**
  (plan estimated 186/25; codebase has grown since).
**Parent plan:** `tasks/unified-beat-engine-phases.md` Phase 1.
**No code changes in this phase.** Every site is tagged with a fate + target.

## Fate taxonomy

| Tag | Meaning | Phase 6 action |
|-----|---------|----------------|
| **U** | **Unify** — 1098 arm is dead post-migration; delete `else`, keep beat arm unconditionally | delete the branch, keep beat arm |
| **K** | **Profile knob** — genuine era-different *outcome*; move to `MechanicsProfile` | replace boolean with profile read |
| **C** | **Clock adapter** — only wall-clock vs `server_ms`; route through the clock seam | route through `now_ms()` / `server_ms` |
| **X** | **Codec/transport** — belongs in `tfs-rust-net`, not core | push to codec |
| **Doc** | Doc comment referencing the flag (no runtime branch) | rewrite prose |
| **Field** | Field declaration / constructor wiring for the flag itself | removed in Phase 6 |
| **Test** | Test-only setup/assertion (not production code) | rewrite after Phase 5 |

## Summary by fate

| Fate | Sites | Notes |
|------|-------|-------|
| **U** | 121 | The vast majority — 1098 reactive arms die under unification |
| **K** | 30 | Genuine era differences → `MechanicsProfile` knobs (see §K rows) |
| **C** | 18 | Wall-clock vs `server_ms` — dissolve into the clock seam |
| **X** | 1 | Player ping opcode — codec |
| **Doc** | 12 | Prose references |
| **Field** | 6 | Flag declaration + constructor |
| **Test** | 16 | Test setup/assertions |

---

## K rows — genuine era differences (require sign-off)

These are the **only** places a real era difference survives. Each becomes a
`MechanicsProfile` knob (or an existing knob). Review and approve each.

### K1 — Parity RNG source

**What:** 772 uses per-world glibc-parity stream (`parity_rng`); 1098 uses env/global or
`ai_rng`. Affects damage/heal rolls, loot, dance choices, shuffle.

**Sites (10):**
- `game_world.rs` L386 `parity_random`, L395 `parity_rand_mod`, L405 `parity_random_shuffle`, L414 `sim_dance_choice`
- `idle_stimulus.rs` L817 `rng_1098`, L963 spell strength, L1002 scaled damage, L1026 heal, L1048 flat delta
- `monster_inventory.rs` L278 loot roll (`beat_driven_loop || sim_glibc_rand_enabled()`)

**Target:** `MechanicsProfile::parity_rng_source` (enum: `PerWorldGlibc` | `EnvGlobal`).
All `parity_random*` / `sim_dance_choice` call sites read the profile, not the boolean.

### K2 — Corpse decay timing

**What:** 772 schedules generic corpse decay at +30 000 ms with 50 ms decay unit; 1098 at
+600 ms with 1 ms decay unit.

**Sites (3):**
- `death.rs` L108 `decay_offset = if beat_driven_loop { 30_000 } else { 600 }`
- `monster_inventory.rs` L450 `decay_clock = if beat_driven_loop { now_ms } else { tick_counter }` (also C)
- `monster_inventory.rs` L455 `decay_unit_ms = if beat_driven_loop { 50 } else { 1 }`

**Target:** `MechanicsProfile::corpse_decay_offset_ms` + `corpse_decay_unit_ms`.
`decay_clock` becomes `now_ms()` unconditionally (C part).

### K3 — Underground visibility z-rule

**What:** 772 `TConnection::IsVisible` (`connections.cc:357-378`) allows underground viewers
to see surface within ±2 floors (no `tz < 8` rejection). 1098/TFS `canSee` rejects
`tz < 8`.

**Sites (8):**
- `game_world_spectators.rs` L60 (param), L72 (`if !beat_driven_loop && tz < 8`)
- `monster_targets.rs` L142, L172, L717 (params to `creature_can_see`)
- `monster_events.rs` L95, L158-159, L201 (params)
- `monster_ai.rs` L2389 (param)

**Target:** `MechanicsProfile::underground_sees_surface: bool`.
`creature_can_see` reads the profile internally; all call sites drop the boolean param.

### K4 — Z-change target clear (monster follow)

**What:** 1098 `Monster::onCreatureMove` clears follow target on z-change; 772
`CreatureMoveStimulus` (`crmain.cc:920`) does NOT — re-arms close-chase instead.

**Sites (1):**
- `monster_events.rs` L209 `let z_change_clears = !self.beat_driven_loop && new_pos.z != old_pos.z`

**Target:** `MechanicsProfile::z_change_clears_follow: bool`.

### K5 — Monster sight-check method

**What:** 772 uses `Map::throw_possible(from, to, 0)` (`crnonpl.cc`); 1098 uses TFS
sight/`canSee` path.

**Sites (1):**
- `monster_targets.rs` L436 `if self.beat_driven_loop { throw_possible } else { ... }`

**Target:** `MechanicsProfile::sight_check_method` (enum: `ThrowPossible` | `TfsCanSee`).

### K6 — Pathfinding parameters (chase)

**What:** 772 `TShortway` uses `max_search_dist=0`, single `FindPathParams` try, reverse
terrain costs, `must_reach` semantics. 1098 TFS uses `max_search_dist=12`, multiple tries,
different direction ordering.

**Sites (5):**
- `monster_ai.rs` L1687 `let tries = if beat_driven_loop { &[fpp] } else { ... }`
- `monster_ai.rs` L1807 `let dirs = if beat_driven_loop { &[N,E,S,W,...] } else { ... }`
- `monster_ai.rs` L1916 `max_search_dist: if beat_driven_loop { 0 } else { 12 }`
- `monster_ai.rs` L2789 `monster_can_occupy_chase_tile` (772 `MovePossible` vs 1098 `tile_query_add`)
- `monster_ai.rs` L2820 `monster_roam_leash_radius` (772 `home_radius` vs global despawn)

**Target:** `MechanicsProfile::chase_pathfinding` sub-struct
(`max_search_dist`, `tries_strategy`, `direction_order`, `tile_occupancy_model`,
`roam_leash_mode`). Most of these are already partly expressed via
`uses_reverse_terrain_path` / `StepSpeedModel`.

### K7 — Keep-distance band (melee distance fighting)

**What:** 772 uses per-type band from `monsters.xml` (`crnonpl.cc` dist branches:
`dist == target_distance`). 1098 uses `dist <= 1`.

**Sites (1):**
- `monster_ai.rs` L353 `if self.beat_driven_loop { return dist == target_distance }`

**Target:** `MechanicsProfile::melee_keep_distance_model` (enum: `PerTypeBand` | `AdjacentOnly`).

### K8 — Monster push/kick model

**What:** 1098 TFS `Monster::pushCreature` (random-cardinal shove, kill on failure).
772 `TMonster::KickCreature` / `KickBoxes` (`crnonpl.cc:3036/2994`).

**Sites (1):**
- `monster_push.rs` L83 `if self.beat_driven_loop { monster_kick_before_step_772 } else { ... }`

**Target:** `MechanicsProfile::monster_push_model` (enum: `Kick772` | `TfsPush`).

### K9 — Drawblood effect emission path

**What:** 1098 emits `CONST_ME_DRAWBLOOD` on the player-notify path
(`game.cpp` combatGetTypeInfo). 772 defers it to the combat apply path
(`crmain.cc:762-775`) to avoid a duplicate draw.

**Sites (1):**
- `game_world_spectators.rs` L309 `if !self.beat_driven_loop { broadcast_magic_effect(DRAWBLOOD) }`

**Target:** `MechanicsProfile::drawblood_on_notify: bool`.

### K10 — Damage text format

**What:** 772 formats "You lose N hitpoints due to an attack by X." 1098 uses a different
format (checked at `else if dmg == 1` branch).

**Sites (1):**
- `game_world_spectators.rs` L328 `let text = if self.beat_driven_loop { ... }`

**Target:** `MechanicsProfile::damage_text_format` (enum or template), or push to codec (X
candidate — but the text is built in core, so K for now).

---

## X rows — codec/transport

### X1 — Player ping opcode

**What:** 772 sends `0x1E` (server-initiated ping-back) for non-OTClient; 1098 sends `0x1D`.

**Sites (1):**
- `player_ping.rs` L65 `if self.beat_driven_loop && !is_otclient { send_ping_back() } else { ... }`

**Target:** move opcode selection into `tfs-rust-net` codec; core only signals "send ping".

---

## C rows — clock adapter (wall-clock vs `server_ms`)

These dissolve into the clock seam: `now_ms()` unconditionally returns `server_ms` after
Phase 6, and `walk_timer_idle` always checks `next_wakeup`.

| File | Line | Context |
|------|------|---------|
| `creature/base.rs` | 196-197 | `walk_timer_idle(beat_driven_loop)` fn itself → always `next_wakeup.is_none()` |
| `walk/mod.rs` | 538 | `walk_timer_idle(self.beat_driven_loop)` call |
| `walk/mod.rs` | 586 | same |
| `walk/mod.rs` | 1266 | same |
| `walk/mod.rs` | 1275 | `server_ms_opt = beat_driven_loop.then_some(server_ms)` → always `Some(server_ms)` |
| `walk/mod.rs` | 1354 | `walk_timer_idle` call |
| `walk/mod.rs` | 1360 | `server_ms_opt` → always `Some(server_ms)` |
| `walk/mod.rs` | 1783 | `beat_driven_loop.then(|| notify_go_ms)` → always compute |
| `idle_stimulus.rs` | 163 | `walk_timer_idle` call |
| `idle_stimulus.rs` | 1229 | `walk_timer_idle` call |
| `monster_ai.rs` | 2171 | `walk_timer_idle` call |
| `monster_ai.rs` | 2200 | `walk_timer_idle` call |
| `monster_inventory.rs` | 450 | `decay_clock = if beat_driven_loop { now_ms } else { tick_counter }` → `now_ms()` (also K2) |
| `game_world.rs` | 161 | `now_ms()` fn itself → always `server_ms` |
| `walk_action.rs` | 89 | `let due = if beat_driven_loop { ... }` → beat arm (server_ms) |
| `monster_events.rs` | 95 | `creature_can_see(... beat_driven_loop)` param (also K3) |
| `monster_events.rs` | 158-159 | same (also K3) |
| `monster_events.rs` | 201 | same (also K3) |

> Note: `creature_can_see` param sites are dual-tagged C+K3 — the call-site routing is C
> (drop the param), the rule selection is K3 (profile knob).

---

## U rows — unify (delete 1098 arm, keep beat arm)

### walk/mod.rs (17 U sites)

| Line | Context | Target |
|------|---------|--------|
| 473 | `if beat_driven_loop && ran_idle` — todo queue drain | keep arm unconditionally |
| 534 | `if beat_driven_loop` — `todo_start_go_delay` beat path | keep arm |
| 595 | `let only_delay = first_step && !beat_driven_loop` | delete 1098 delay-only |
| 623 | `commit_next_walk_deadline` early return | delete fn (Phase 5) |
| 634 | `sync_walk_timer_arm` early return | delete fn (Phase 5) |
| 738 | `player_todo_clear_with_snapback` early return when not beat | always run |
| 782 | beat arm for `CGoDirection` todo path | keep arm |
| 841 | beat arm for `CGoPath` todo path | keep arm |
| 964 | `player_stop_auto_walk` beat arm (OTClient workaround) | keep arm |
| 1295 | beat arm `schedule_creature_wakeup` vs else | keep beat arm |
| 1392 | beat arm for `schedule_creature_wakeup` | keep arm |
| 1402 | beat arm for schedule | keep arm |
| 1556 | beat arm for `EarliestWalkTime` | keep arm |
| 1608 | `if d.is_some() && beat_driven_loop` — pop walk_destinations | keep arm |
| 1801 | beat arm for `last_step_server_ms` | keep arm |
| 1826 | beat arm for `stop_event_walk` followup | keep arm |
| 1895 | beat arm for reschedule | keep arm |
| 2091 | `if !beat_driven_loop` — 1098 `nextAction` lockout | delete (ToDo delay handles it) |
| 2098 | beat arm for chase debug | keep (debug) |

### idle_stimulus.rs (20 U sites)

| Line | Context |
|------|---------|
| 93 | `idle_stimulus` gate — always run |
| 154 | `request_idle_stimulus` gate — always run |
| 248 | beat arm for blood pool on damage |
| 271 | `damage_stimulus` gate — always run |
| 340 | `if !has_target && !beat_driven_loop` — 1098 chase acquire → idle arm |
| 361 | `on_creature_move_stimulus` gate — always run |
| 903 | `if !beat_driven_loop || dist > 1` — suppress adjacent melee spell |
| 1137 | `if beat_driven_loop` — `idle_stimulus_last_ms` |
| 1163 | `beat_driven_loop && m.state == Sleeping` — param |
| 1170 | `if beat_driven_loop && is_summon` — summon lifecycle |
| 1182 | `else if !beat_driven_loop && is_idle` — 1098 idle return |
| 1186 | `if beat_driven_loop` — `lose_existing_target` |
| 1205 | `else if !beat_driven_loop && has_opponents` — 1098 search target |
| 1219 | `if !beat_driven_loop` — `on_think_target` (1098 only) |
| 1425 | `set_combat_chase_mode` gate — always run |
| 1471 | `emit_combat_state` gate — always run |
| 1630 | `if !beat_driven_loop` — `can_use_attack` fallback |
| 1656 | `rotate_toward_attack` gate — always run |
| 1811 | `maybe_enqueue_at_goal_wait` gate — always run |
| 2190 | `if beat_driven_loop` — `prepare_and_enqueue_go` |
| 2202 | `if beat_driven_loop` — `emit_combat_state` |
| 2221 | `if !beat_driven_loop` — `keep_dance_walk_alive` (1098 only) |

### monster_ai.rs (24 U sites)

| Line | Context |
|------|---------|
| 302 | `if beat_driven_loop` — chase queue stale check |
| 409 | `if !beat_driven_loop return` |
| 594 | `melee_realign && !beat_driven_loop` — 1098 melee realign |
| 786 | `if beat_driven_loop` — stall rescue |
| 850 | `debug_assert!(beat_driven_loop)` — becomes canonical |
| 1041 | `if !beat_driven_loop return` — `skip_idle_melee_chase` |
| 1061 | `if !beat_driven_loop || target_distance > 1` |
| 1073 | `if !beat_driven_loop return` |
| 1107 | `if !beat_driven_loop return` |
| 1213 | `if !beat_driven_loop return` — `chase_stalled_without_wakeup` |
| 1252 | `if !beat_driven_loop return` — `combat_scheduler_needs_refresh` |
| 1470 | `if beat_driven_loop` — `go_to_follow_creature` beat arm |
| 1547-1549 | `!beat_driven_loop && use_distance_step` |
| 1702 | `else if beat_driven_loop` — 772 `ToDoGo` trim |
| 1720 | `if beat_driven_loop && chase_debug` (debug) |
| 1752 | `if beat_driven_loop && steps.is_empty` |
| 1765 | `if !beat_driven_loop` — `monster_start_chase_walk` |
| 1770 | `if beat_driven_loop && chase_debug` (debug) |
| 1874 | `if beat_driven_loop` — `monster_start_chase_walk` |
| 1926 | `else if beat_driven_loop` |
| 1938 | `else if beat_driven_loop` |
| 1970 | `debug_assert!(!beat_driven_loop || uses_reverse_terrain)` |
| 1974 | `if uses_reverse_terrain && beat_driven_loop` |
| 2069 | `if beat_driven_loop return` — `on_think_target` (1098 only) |
| 2269 | `if beat_driven_loop return` — `monster_maybe_walk_to_spawn` (1098 only) |
| 2333 | `if had_follow_path && !beat_driven_loop` |
| 2398 | `&& !beat_driven_loop` |
| 2429 | `if beat_driven_loop` — idle drain owns flee |

### monster_events.rs (10 U sites)

| Line | Context |
|------|---------|
| 35 | `if beat_driven_loop` — `request_idle_stimulus` vs `searchTarget` |
| 149 | `if beat_driven_loop` — `sleep_wake_on_creature_move` |
| 179 | `if beat_driven_loop` — follow target pos |
| 231 | `if beat_driven_loop` — `schedule_chase_after_opponent` |
| 238 | `if beat_driven_loop` — follow repath |
| 268 | `!has_path && !follow_repath_without_path && !beat_driven_loop` |
| 280 | `let should_repath = if beat_driven_loop` |
| 302 | `if beat_driven_loop` — `todo.queue.clear` |
| 315 | `if beat_driven_loop` — `idle_stimulus_after_creature_move` |
| 345 | `if !beat_driven_loop return` |
| 435 | `if !beat_driven_loop return` |

### game_loop.rs (6 U sites)

| Line | Context |
|------|---------|
| 277 | `if beat_driven_loop` — `player_reset_connection_rounds` |
| 418 | `if beat_driven_loop` — ToDo `UseItem` path |
| 454 | `if beat_driven_loop` — ToDo `UseItem` path |
| 477 | `if beat_driven_loop` — ToDo `UseItemEx` path |
| 511 | `if beat_driven_loop` — ToDo `LookAt` path |
| 578 | `if !beat_driven_loop` — `process_walk_deadlines` (delete) |

### game_world.rs (6 U sites)

| Line | Context |
|------|---------|
| 183 | `if !beat_driven_loop` — `timed_action_ready` |
| 209 | same |
| 225 | `if beat_driven_loop` — `walk_action_ready_at` |
| 237 | `if beat_driven_loop` — `multiuse_ready_at` |
| 247 | `if !beat_driven_loop return` — `player_apply_multiuse_exhaust` |
| 260 | `if !beat_driven_loop return` — `player_apply_spell_exhaust` |

### monster_targets.rs (6 U sites)

| Line | Context |
|------|---------|
| 232 | `else if beat_driven_loop` — `request_idle_stimulus` |
| 247 | `if beat_driven_loop` — `request_idle_stimulus` |
| 375 | `if beat_driven_loop` — Sleeping state |
| 400 | `if beat_driven_loop` — `request_idle_stimulus` |
| 672 | `if ret && !beat_driven_loop` — look direction on `selectTarget` |
| 736 | `if beat_driven_loop` — `arm_idle` |

### walk_action.rs (3 U sites)

| Line | Context |
|------|---------|
| 45 | `if beat_driven_loop return` — `on_player_walk_complete` (1098 only) |
| 63 | `if beat_driven_loop return` — `process_walk_action_tasks` (1098 only) |
| 105 | `if beat_driven_loop` — `schedule_creature_wakeup` |

### creature_todo.rs (5 U sites)

| Line | Context |
|------|---------|
| 65 | trace field `beat_driven = world.beat_driven_loop` |
| 617 | `if beat_driven_loop && chase_debug` (debug) |
| 655 | `if beat_driven_loop` — `todo_start_go_delay` |
| 679 | `if !beat_driven_loop return` — `creature_todo_yield` |
| 704 | `beat_driven_loop && creatures.get` — `creature_uses_todo_execute` check |

### Other U sites

| File | Line | Context |
|------|------|---------|
| `spawn_lifecycle.rs` | 343 | `if beat_driven_loop` — monster experience/target on spawn |
| `game_world_tick.rs` | 16 | `if walk_wake_tx.is_none() && !beat_driven_loop` — `process_walk_deadlines` |
| `game_world_tick.rs` | 44 | `if beat_driven_loop` — `round_nr_772` / `process_connections_772` |
| `game_world_lifecycle.rs` | 173 | `corpse_snapshot = if beat_driven_loop` |
| `creature_think.rs` | 242 | `if !beat_driven_loop` — follow repath |
| `process_skills.rs` | 28 | `if beat_driven_loop` — `process_player_fed_regen_772` |
| `player_combat.rs` | 87 | `if !beat_driven_loop return` — 1098 path defer |
| `player_combat.rs` | 198 | `if !beat_driven_loop return` — `player_cancel_attack_and_follow` |
| `connections_772.rs` | 37 | `if !beat_driven_loop return` — `packet_counts_as_action` |
| `connections_772.rs` | 56 | `if !beat_driven_loop return` — `process_connections_772` |
| `connections_772.rs` | 105 | `if !beat_driven_loop return` — `tick_ambiente_light_772` |
| `game_world_spectators.rs` | 309 | `if !beat_driven_loop` — drawblood (also K9) |
| `spell.rs` | 43 | `if beat_driven_loop` — `spell_ready_at` check |

---

## Doc rows (12)

| File | Line | Context |
|------|------|---------|
| `idle_stimulus.rs` | 6 | module doc |
| `game_loop.rs` | 931 | test module doc |
| `creature/base.rs` | 13 | `ChaseMode` doc |
| `creature/monster.rs` | 22 | `MonsterState` doc |
| `monster_push.rs` | 3, 5 | module doc |
| `monster_ai.rs` | 2784 | `monster_can_occupy_chase_tile` doc |
| `game_world_spectators.rs` | 51, 819 | `creature_can_see` doc + test doc |

---

## Field rows (6)

| File | Line | Context |
|------|------|---------|
| `game_world.rs` | 117 | `pub(crate) beat_driven_loop: bool` field |
| `game_world.rs` | 288 | constructor `beat_driven_loop = step_speed == LinearGo` |
| `game_world.rs` | 326 | field init in struct literal |
| `death.rs` | 53 | `beat_driven_loop: bool` param |
| `spell.rs` | 41 | `beat_driven_loop: bool` param |
| `game_world_lifecycle.rs` | 219-220 | `!beat_driven_loop, beat_driven_loop` params to death fn |

---

## Test rows (16)

| File | Line | Context |
|------|------|---------|
| `sim_harness.rs` | 471, 995, 1207, 1254 | harness setup |
| `game_loop.rs` | 917, 920, 922, 923, 1148 | test fn + assertions |
| `idle_stimulus_tests.rs` | 402, 2469 | test assertions |
| `monster_inventory.rs` | 624 | test setup |
| `monster_ai_world_tests.rs` | 22 | test setup |

---

## Cross-reference: K knobs → `MechanicsProfile` fields

| Knob | Profile field (proposed) | Type |
|------|--------------------------|------|
| K1 | `parity_rng_source` | enum `PerWorldGlibc` \| `EnvGlobal` |
| K2 | `corpse_decay_offset_ms` + `corpse_decay_unit_ms` | `u64` + `u64` |
| K3 | `underground_sees_surface` | `bool` |
| K4 | `z_change_clears_follow` | `bool` |
| K5 | `sight_check_method` | enum `ThrowPossible` \| `TfsCanSee` |
| K6 | `chase_pathfinding` sub-struct | struct |
| K7 | `melee_keep_distance_model` | enum `PerTypeBand` \| `AdjacentOnly` |
| K8 | `monster_push_model` | enum `Kick772` \| `TfsPush` |
| K9 | `drawblood_on_notify` | `bool` |
| K10 | `damage_text_format` | enum / template |

Several of these (K6 `uses_reverse_terrain_path`, K7, K1) already have partial profile
representation. Phase 2 will consolidate them.

---

## Exit criteria

- [x] Every `beat_driven_loop` site tagged U / K / C / X / Doc / Field / Test
- [x] K rows signed off by user
- [ ] K knob names confirmed against existing `MechanicsProfile` fields (Phase 2 prep)
