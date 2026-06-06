# TFS-RUST: Refactoring Opportunities

**Date**: 2026-06-06 (audited against live repo)  
**Scope**: All crates, ranked by urgency.

---

## 1. ~~`monster_ai.rs` — 2,726 lines~~ ✅ Done (2026-06-06)

Split into three `impl GameWorld` extension modules (layout-only; no behavior change):

| File | Lines | Contents |
|---|---|---|
| [`monster_targets.rs`](../crates/tfs-rust-core/src/monster_targets.rs) | ~576 | Friend/opponent lists, idle status, target search/select/follow |
| [`monster_events.rs`](../crates/tfs-rust-core/src/monster_events.rs) | ~258 | Spectator fan-out, creature move/appear reactions |
| [`monster_ai.rs`](../crates/tfs-rust-core/src/monster_ai.rs) | ~1,963 | Think loop, chase/follow pathing, spawn return, tests |

Cross-module helpers are `pub(crate)` (Rust privacy is per-module, not per-`impl`).

<details>
<summary>Original extraction plan (completed)</summary>

### Extract → `monster_targets.rs` (~650–700 lines)

Friend/opponent tracking and target selection:

- `monster_update_target_list`, `monster_prune_creature_lists`, `monster_remove_creature_from_lists`
- `monster_is_friend`, `monster_is_opponent`, `monster_add_friend`, `monster_add_opponent`, `monster_ensure_opponent_listed`
- `monster_search_target`, `monster_select_target`, `monster_set_follow_creature`
- `monster_can_use_attack`, `monster_update_idle_status`, `monster_set_idle`, `monster_is_target`
- `monster_schedule_chase_after_opponent_add`, `monster_try_acquire_chase_target`

### Extract → `monster_events.rs` (~300 lines)

Creature appear/move reactions:

- `monster_on_creature_move`, `monster_on_creature_appear_self`
- `monster_on_follow_creature_moved`, `monster_on_follow_creature_complete`
- `monster_notify_creature_enter_viewport`, `monster_dispatch_creature_move`
- `monsters_witnessing_move`

</details>

**Next split:** [`game_world.rs`](crates/tfs-rust-core/src/game_world.rs) (2,520 lines) — orchestration, tick/beat advance, broadcasts, creature lifecycle.

---

## 2. ~~Duplicate distance helpers~~ ✅ Done (2026-06-06)

`distance_x`, `distance_y`, `offset_x`, `offset_y` are `pub(crate)` in [`monster_distance_step.rs`](../crates/tfs-rust-core/src/monster_distance_step.rs). Duplicates removed from `monster_ai.rs`.

Look-direction uses argument swap: C++ `getOffsetX(attackedCreaturePos, pos)` → `offset_x(target, from)` (distance-step convention is `creature − target`).

---

## 3. `walk.rs` — 2,593 lines (lower priority, plan ahead)

Single coherent domain — no urgency — but the top ~950 lines are module-level free functions covering two distinct concerns worth splitting before combat/conditions add to the file.

### Extract → `walk_timing.rs` (~400 lines)

Pure speed/timing functions:

- `calculated_step_speed_tfs`, `get_step_duration`, `get_walk_delay`, `get_event_step_ticks`
- `walk_timing_speed`, `tfs_retail_log_speed`, `balanced_softened_go`, `cipsoft_speed_from_profile`
- `walk_timing_speed_kind`, `step_speed_for_walk`, `go_strength_for_walk`

### Extract → `walk_tile.rs` (~440 lines)

Tile traversal checks:

- `tile_query_add_monster`, `tile_query_add_npc`, `tile_query_add_player`
- `resolve_player_move_destination`, `query_destination`

*(Original ~300-line estimate was low — live block is ~440 lines.)*

### What remains in `walk.rs`

Direction utils, `impl GameWorld` block (~1,100 lines). Tests stay with `walk.rs`.

---

## 4. `tfs-rust-content/items.rs` — 1,336 lines

OTB binary loading and XML attribute parsing are two distinct parsers sharing a file. Everything from `apply_xml_attribute` (~line 604) downward is pure XML attribute dispatch with no shared state with the OTB code above it.

**Fix**: extract to `item_xml.rs` in `tfs-rust-content`.  
Lower priority — content loading code, not game logic — but it will be touched whenever new item attributes are added.

---

## 5. `player_inventory_query_add.rs` — 1,140 lines

Fourth-largest file in `tfs-rust-core`. Coherent `queryAdd` / dress / slot-mask domain — candidate for split only if inventory grows further (e.g. separate equip-check vs query-max-count arms). Lower urgency than `monster_ai` / `game_world`.

---

## 6. `tfs-rust-net/outgoing_extra.rs` — 1,048 lines (flag now, split later)

Currently a manageable collection of small stateless packet builder functions. Will bloat fast as combat, channels, and conditions land.

**Planned split** — when the first combat `send_*` batch arrives:

| File | Contents |
|---|---|
| `outgoing_creature.rs` | Outfit, skull, shield, speed, walkthrough, light |
| `outgoing_channel.rs` | Channel open/close/events, private channels |
| `outgoing_combat.rs` | Distance shoot, magic effects, health bars |

No action needed now — add a `// TODO: split outgoing_extra.rs when combat packets land` comment as a reminder.

---

## Not on the list (already good shape)

| Module | Notes |
|---|---|
| `creature/`, `combat/`, `map/` | Already directory modules with well-sized subfiles |
| `game_world_inventory.rs` (966 lines) | Large but coherent |
| `container_ops.rs` (757 lines) | Focused domain, correct shape |
| `monster_distance_step.rs` (635 lines) | Pure functions, no `impl GameWorld`, exemplary shape |
| `monster_targets.rs` (~576 lines) | Target list / search — split from `monster_ai` (M1, 2026-06-06) |
| `monster_events.rs` (~258 lines) | Move/appear fan-out — split from `monster_ai` (M1, 2026-06-06) |

The pattern that's already working in `monster_distance_step.rs` and `container_ops.rs` is the target shape — now applied to `monster_targets.rs` and `monster_events.rs` (M1).

---

## Refactoring vs game loop

File-layout refactors (`monster_ai` split, distance helper dedupe, `walk.rs` split) **do not change** game loop architecture or runtime behavior.
