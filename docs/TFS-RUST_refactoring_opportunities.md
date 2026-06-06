# TFS-RUST: Refactoring Opportunities

**Date**: 2026-06-06 (audited against live repo)  
**Scope**: All crates, ranked by urgency.

---

## 1. `monster_ai.rs` — 2,649 lines ⚠️ Most urgent

Larger than `game_world.rs` and growing. Three coherent sub-domains can be extracted using the same `impl GameWorld` extension pattern.

### Extract → `monster_targets.rs` (~400 lines)

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

### What remains in `monster_ai.rs`

Think loop + chase/follow logic + spawn return + tests (~1,000 lines). Manageable.

---

## 2. Duplicate distance helpers — violates refactor rule 1

`monster_ai.rs` and `monster_distance_step.rs` both privately define:

```
distance_x, distance_y, offset_x, offset_y
```

**Fix**: make the four functions `pub(crate)` in `monster_distance_step.rs`, delete the copies from `monster_ai.rs`. Two-minute PR, zero risk.

---

## 3. `walk.rs` — 2,257 lines (lower priority, plan ahead)

Single coherent domain — no urgency — but the top ~950 lines are module-level free functions covering two distinct concerns worth splitting before combat/conditions add to the file.

### Extract → `walk_timing.rs` (~400 lines)

Pure speed/timing functions:

- `calculated_step_speed_tfs`, `get_step_duration`, `get_walk_delay`, `get_event_step_ticks`
- `walk_timing_speed`, `tfs_retail_log_speed`, `balanced_softened_go`, `cipsoft_speed_from_profile`
- `walk_timing_speed_kind`, `step_speed_for_walk`, `go_strength_for_walk`

### Extract → `walk_tile.rs` (~300 lines)

Tile traversal checks:

- `tile_query_add_monster`, `tile_query_add_npc`, `tile_query_add_player`
- `resolve_player_move_destination`, `query_destination`

### What remains in `walk.rs`

Direction utils, `impl GameWorld` block (~1,100 lines). Tests stay with `walk.rs`.

---

## 4. `tfs-rust-content/items.rs` — 1,336 lines

OTB binary loading and XML attribute parsing are two distinct parsers sharing a file. Everything from `apply_xml_attribute` (~line 604) downward is pure XML attribute dispatch with no shared state with the OTB code above it.

**Fix**: extract to `item_xml.rs` in `tfs-rust-content`.  
Lower priority — content loading code, not game logic — but it will be touched whenever new item attributes are added.

---

## 5. `tfs-rust-net/outgoing_extra.rs` — 1,048 lines (flag now, split later)

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

The pattern that's already working in `monster_distance_step.rs` and `container_ops.rs` is exactly the target shape for `monster_targets.rs` and `monster_events.rs`.
