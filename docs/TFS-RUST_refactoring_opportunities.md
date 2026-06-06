# TFS-RUST: Refactoring Opportunities

**Date**: 2026-06-06 (audited against live repo)  
**Scope**: All crates, ranked by urgency.

File-layout refactors **do not change** game loop architecture or runtime behavior.

**Target shape:** small focused modules like `monster_distance_step.rs` and `container_ops.rs` — `impl GameWorld` extensions where needed, `pub(crate)` cross-module helpers.

---

## Open (ranked)

### 1. `tfs-rust-content/items.rs` — ~1,336 lines

OTB binary loading and XML attribute parsing are two distinct parsers sharing a file. Everything from `apply_xml_attribute` (~line 604) downward is pure XML attribute dispatch with no shared state with the OTB code above it.

**Fix:** extract to `item_xml.rs` in `tfs-rust-content`.  
Lower priority — content loading, not game logic — but touched whenever new item attributes land.

---

### 2. `player_inventory_query_add.rs` — ~1,140 lines

Coherent `queryAdd` / dress / slot-mask domain. Split only if inventory grows further (e.g. separate equip-check vs query-max-count arms). Lower urgency than remaining `game_world_*` item modules.

---

### 3. `tfs-rust-net/outgoing_extra.rs` — ~1,048 lines (flag now, split later)

Manageable collection of small stateless packet builders today; will bloat as combat, channels, and conditions land.

**Planned split** — when the first combat `send_*` batch arrives:

| File | Contents |
|---|---|
| `outgoing_creature.rs` | Outfit, skull, shield, speed, walkthrough, light |
| `outgoing_channel.rs` | Channel open/close/events, private channels |
| `outgoing_combat.rs` | Distance shoot, magic effects, health bars |

No action now — add a `// TODO: split outgoing_extra.rs when combat packets land` comment when that work starts.

---

## Completed (2026-06-06)

### `walk.rs` split

Layout-only split of the walking module into a directory anchor + two pure-function child modules. External `crate::walk::*` import paths unchanged (`wire_step_speed`, `WalkSpeedRole`, `tile_query_add_creature`, etc.).

| File | Lines | Contents |
|---|---|---|
| [`walk/mod.rs`](../crates/tfs-rust-core/src/walk/mod.rs) | ~1,789 | Flags, direction utils, drunk walk, teleport, `tile_query_add_creature` dispatch, `impl GameWorld`, tests |
| [`walk/walk_timing.rs`](../crates/tfs-rust-core/src/walk/walk_timing.rs) | ~380 | Speed/timing: `get_step_duration`, `get_event_step_ticks`, `wire_step_speed`, CipSoft/TFS curves |
| [`walk/walk_tile.rs`](../crates/tfs-rust-core/src/walk/walk_tile.rs) | ~462 | `Tile::queryAdd` arms, `queryDestination`, height floor-change resolution |

### `monster_ai.rs` split

Layout-only split into three `impl GameWorld` extension modules:

| File | Lines | Contents |
|---|---|---|
| [`monster_targets.rs`](../crates/tfs-rust-core/src/monster_targets.rs) | ~576 | Friend/opponent lists, idle status, target search/select/follow |
| [`monster_events.rs`](../crates/tfs-rust-core/src/monster_events.rs) | ~258 | Spectator fan-out, creature move/appear reactions |
| [`monster_ai.rs`](../crates/tfs-rust-core/src/monster_ai.rs) | ~1,963 | Think loop, chase/follow pathing, spawn return, tests |

Cross-module helpers are `pub(crate)` (Rust privacy is per-module, not per-`impl`).

### Duplicate distance helpers

`distance_x`, `distance_y`, `offset_x`, `offset_y` consolidated in [`monster_distance_step.rs`](../crates/tfs-rust-core/src/monster_distance_step.rs); duplicates removed from `monster_ai.rs`.

Look-direction uses argument swap: C++ `getOffsetX(attackedCreaturePos, pos)` → `offset_x(target, from)` (distance-step convention is `creature − target`).

### `game_world.rs` split

Layout-only split of the residual monolith (alongside existing `game_world_inventory` / `game_world_save`):

| File | Lines | Contents |
|---|---|---|
| [`game_world.rs`](../crates/tfs-rust-core/src/game_world.rs) | ~213 | `GameWorld` struct, `new`, hooks, re-exports |
| [`game_world_tick.rs`](../crates/tfs-rust-core/src/game_world_tick.rs) | ~66 | `on_tick`, `advance_beat_772`, subsystem polling |
| [`game_world_lifecycle.rs`](../crates/tfs-rust-core/src/game_world_lifecycle.rs) | ~169 | Release/cleanup, remove, logout, death |
| [`game_world_spectators.rs`](../crates/tfs-rust-core/src/game_world_spectators.rs) | ~404 | Visibility, known-set, output queue, tile item broadcasts |
| [`game_world_item_cylinder.rs`](../crates/tfs-rust-core/src/game_world_item_cylinder.rs) | ~304 | Cylinder resolve, tile add/remove |
| [`game_world_item_move.rs`](../crates/tfs-rust-core/src/game_world_item_move.rs) | ~733 | `internal_move_item` cylinder match tree |
| [`game_world_player_throw.rs`](../crates/tfs-rust-core/src/game_world_player_throw.rs) | ~369 | `playerMoveThing` / throw geometry |
| [`game_world_player.rs`](../crates/tfs-rust-core/src/game_world_player.rs) | ~130 | Stats packet, group flags, capacity |
| [`game_world_script.rs`](../crates/tfs-rust-core/src/game_world_script.rs) | ~230 | `ScriptContext` trait impl |

`creature_can_see` / `protocol_can_see` re-exported from the anchor for stable import paths.

---

## Not on the list (already good shape)

| Module | Notes |
|---|---|
| `creature/`, `combat/`, `map/` | Directory modules with well-sized subfiles |
| `game_world_inventory.rs` (~966 lines) | Large but coherent |
| `game_world_item_move.rs` (~733 lines) | Cylinder match tree — split from anchor |
| `container_ops.rs` (~757 lines) | Focused domain, correct shape |
| `monster_distance_step.rs` (~635 lines) | Pure functions, no `impl GameWorld`, exemplary shape |
| `monster_targets.rs`, `monster_events.rs` | Split from `monster_ai` (see Completed) |
| `walk/walk_timing.rs`, `walk/walk_tile.rs` | Split from `walk/mod.rs` (see Completed) |
