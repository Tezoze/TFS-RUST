# Map System Audit — Chunk Grid vs 772 Reference

**Date:** 2026-07-02
**Scope:** Everything the running server does with the map — sparse chunk grid storage, tile
model + OTBM ingest, line-of-sight / throw, spectator fan-out (monster AI + player packets),
and the `GetMapDescription` wire encoder.
**Rust side:**
- `crates/tfs-rust-core/src/map/grid.rs` — `SparseGrid`, `Chunk`, `ChunkKey`, `collect_spectators`
- `crates/tfs-rust-core/src/map/mod.rs` — `Map`, OTBM ingest, flag conversion, `blocks_sight`
- `crates/tfs-rust-core/src/map/los.rs` — `is_sight_clear`, `can_throw_to`, `throw_possible`
- `crates/tfs-rust-core/src/tile.rs` — `TileBody`, `flags`, `map_object_chain`
- `crates/tfs-rust-core/src/game_world_spectators.rs` — `spectator_conns`, `broadcast_*`
- `crates/tfs-rust-core/src/monster_events.rs` — `collect_spatial_spectators`
- `crates/tfs-rust-net/src/map_description.rs` — `GetTileDescription` / `GetMapDescription`
**Reference:** `reference/cipsoft-772/tibia-game-master/src/` — `map.hh`, `map.cc`, `info.cc`,
`crmain.cc`; repo-root `src/` for 1098 TFS parity.

Findings are graded: **BUG** (implemented but diverges), **GAP** (reference behavior absent /
design shortfall), **SUSPECT** (probable divergence, verify before fixing). Severity is
**HIGH / MED / LOW** by gameplay + protocol impact.

---

## Summary table

| # | Sev | Kind | Area | Finding | Status |
|---|-----|------|------|---------|--------|
| 1 | HIGH | BUG | LOS | `blocks_sight` checks `BLOCKSOLID\|BLOCKPATH`, never the projectile-block flag (`UNTHROW`) | ✅ Phase 1 |
| 2 | HIGH | BUG | Wire | `GetTileDescription` creature loop has no 10-thing stack cap → client stack desync | ✅ Phase 1 (era-corrected) |
| 3 | MED | GAP | Storage | Creature placed on a void (unloaded) tile is silently dropped from tile + chunk index | ✅ Phase 2 |
| 4 | HIGH | GAP | Fan-out | Player packet fan-out (`spectator_conns`) linear-scans all connections, ignores the grid | Phase 3 |
| 5 | MED | GAP | Fan-out | 64×64 chunk is 4× coarser than 772's 16×16 creature blocks (over-collect + tie-break order) | Phase 4 |
| 6 | MED | SUSPECT | LOS | `blocks_sight` returns `true` for missing tiles; 1098 `checkSightLine` treats null tiles as non-blocking | ✅ Phase 1 |
| 7 | LOW | GAP | Storage | Dual creature lists (`TileBody.creatures` + `Chunk.creatures`) can desync via direct grid calls | ✅ Phase 2 |
| 8 | LOW | GAP | Query | `find_item_position` full-world scan (every chunk × 4096 slots) | Phase 5 |
| 9 | LOW | SUSPECT | Tile | `is_walkable` ignores creatures + `BLOCKPATH`; `Tile::query_add` is a `true` stub | Phase 5 |

---

## Architecture context — chunk grid vs 772 sectors

**772 (`map.hh`).** The CipSoft map is a grid of **32×32 `TSector`s** per floor, each cell holding
the head `Object` of a **priority-ordered linked list** (`PRIORITY_BANK` 0 → `CLIP` → `BOTTOM` →
`TOP` → `CREATURE` → `LOW` 5), walked with `GetFirstObject`/`getNextObject`. Sectors are
disk-swappable (`STATUS_LOADED`/`STATUS_SWAPPED`, `SwapSector`/`UnswapSector`) — they stream from
`.sec` files and swap out under memory pressure. A **separate 16×16-block creature index**
(`TFindCreatures`, `crmain.cc`) is used for spectator/creature search, distinct from the 32×32
tile sectors.

**Rust (`grid.rs`).** `FxHashMap<ChunkKey, Box<Chunk>>`, each `Chunk` a 64×64 floor region with a
`Box<[Option<Box<Tile>>; 4096]>` tile array **and** a per-chunk `SmallVec<[CreatureId; 4]>` spatial
index. Chunks are lazily allocated on first `insert_tile`. The full OTBM stays resident; there is
no swap-to-disk.

**Verdict:** the OTBM-in-RAM chunk grid is the correct modern replacement and is *better* than the
772 sector-swap system for in-game experience:

- **No swap stalls.** 772's sector swapping causes latency spikes on cold sectors; a resident map
  gives flat, predictable tile-access latency. The swap machinery existed only for 2004-era RAM.
- **Sparse allocation** preserves the "only pay for populated space" property of sectors without a
  disk round-trip.
- **Priority chain preserved.** The 772 `PRIORITY_*` linked list is faithfully reproduced by the
  `ground → top_items → creatures → down_items` split (`tile.rs::map_object_chain`).

Keeping OTBM instead of `.sec` is the right decision. The in-game experience is equivalent once the
LOS and stack-cap bugs (below) are fixed. The only structural gap vs 772 is that a *single* 64×64
chunk serves both tiles and creatures, where 772 uses 32×32 tile sectors + 16×16 creature blocks
(finding #5).

---

## 1. `blocks_sight` checks the wrong flag — HIGH, BUG — ✅ FIXED

- **Rust:** `map/mod.rs:85` —
  ```rust
  body.flags & (flags::BLOCK_SOLID | flags::BLOCK_PROJECTILE) != 0
  ```
  `BLOCK_PROJECTILE` is a legacy alias for `BLOCKPATH` (1<<18) in `tile.rs:62`. The actual
  projectile-blocking property is `UNTHROW` (1<<24), set from `item_type.block_projectile()` in
  `apply_item_tile_flags` (`map/mod.rs`). So `is_sight_clear` never consults the projectile-block
  flag — it tests pathfinding-block instead.
- **1098:** `Map::checkSightLine` (repo-root `src/map.cpp`) blocks sight **only** on
  `CONST_PROP_BLOCKPROJECTILE` — not `blockPathFind`, not `blockSolid`.
- **772:** `throw_possible` (`los.rs`) already correctly tests `UNTHROW` (`info.cc` `ThrowPossible`).
  The two LOS paths currently disagree on what blocks.
- **Impact:** items that block pathing but not projectiles (tables, low fences, many decorations)
  wrongly block line of sight; solid furniture flagged `blockSolid` but not `blockProjectile`
  wrongly blocks sight. This drives real gameplay:
  - `monster_ai.rs:1566`, `monster_ai.rs:1864` — distance-step / kiting / flee decisions (**both eras**).
  - `pathfinding.rs:904` — `clear_sight` path goals.
- **Fix:** `body.flags & flags::UNTHROW != 0`. See also finding #6 for the `None` branch.

## 2. Missing creature stack cap in the wire encoder — HIGH, BUG — ✅ FIXED (era-corrected)

- **Rust:** `map_description.rs::get_tile_description` (and its `count_tile_description` twin) caps
  `top_items` and `bottom_items` at 10 things, but the **creature loop had no cap**:
  ```rust
  for c in tile.creatures.iter().rev() {
      ...
      codec.write_add_creature(msg, &cw);
      count += 1;            // no `if count == 10` guard
  }
  ```
  Additionally the top-items loop uses `continue` at `count == 10` where C++ `return`s — so once 10
  is reached in top items, remaining top items are skipped but creatures were still all emitted.
- **1098:** `ProtocolGame::GetTileDescription` (repo-root `src/protocolgame.cpp:645-693`) —
  **era correction:** the creature loop increments `count` but does **not** check it; only the
  top-items `break` and down-items `return` enforce the 10-thing cap. The original audit claim that
  1098 "breaks at exactly 10 things across ground + top + creatures + down" was incorrect.
- **772:** `ProtocolGame::GetTileDescription` (`gameserver/src/protocolgame.cpp:539-587`) — the
  creature loop **does** `if (++count == 10) return;` early-exit, matching the down-items return.
- **Impact:** on a 772 tile with >10 things, the classic client reads only 10 objects, then
  interprets the 11th thing's bytes as the next-tile skip count → **stack desync / corrupted floor
  render**. Low frequency (needs a crowded tile) but a hard 772 protocol violation. 1098 was not
  affected (its C++ also does not cap creatures).
- **Fix (applied):** added `tile_description_caps_creatures() -> bool` to `ProtocolCodec` —
  `false` for 1098, `true` for 772 — threaded through both `get_tile_description` and
  `count_tile_description` (the two must stay byte-identical or the `count_map_description_body`
  debug assert trips).

## 3. Silent creature-registration loss on void tiles — MED, GAP

- **Rust:** `map/mod.rs:96` `register_creature_at` only mutates the tile list when a tile exists;
  `grid.rs:127` `register_creature` no-ops when the chunk is absent. A creature placed at a
  position with no loaded tile is silently absent from both the tile stack and the chunk spatial
  index — invisible to spectator queries and monster AI.
- **Impact:** hard-to-diagnose "ghost" creatures if any spawn/teleport path ever targets a void
  tile. Currently masked by the invariant that creatures stand on valid tiles.
- **Fix:** make the invariant explicit — `debug_assert!` the chunk/tile exists, or return a
  `Result`/log at registration, rather than silently dropping.

## 4. Player packet fan-out ignores the spatial index — HIGH, GAP

- **Rust:** `game_world_spectators.rs::spectator_conns` iterates `conn_to_creature` for **every**
  broadcast (`broadcast_to_spectators`, magic effects, health bars, tile add/update/remove,
  creature-say). That is O(players) per event, O(players²) aggregate for map-wide activity.
- **Reference:** both 772 (`TFindCreatures`, 16×16 blocks) and 1098 (`Map::getSpectators`,
  quadtree) resolve spectators spatially, never by scanning the full online set.
- **Impact:** at 2000+ players this is the scaling ceiling. The grid already indexes creatures
  spatially and players are creatures in it; only the *player* path fails to use it.
- **Fix:** resolve spectators via `grid.collect_spectators` → filter to creatures holding a
  `ConnId` → `protocol_can_see`. Turns fan-out from O(all players) into O(local crowd). The monster
  path (`collect_spatial_spectators`) already demonstrates the pattern.

## 5. Chunk granularity vs 772 creature blocks — MED, GAP

- **Rust:** one 64×64 chunk serves both tiles and the creature index. `collect_spectators` walks
  overlapping 64×64 chunks; a range-11 query (23×23 box) can pull creatures from a ~128×128 span,
  then `collect_spatial_spectators` sorts + dedups + `canSee`-filters every call.
- **772:** 32×32 tile sectors **and** a separate 16×16 creature block index (`TFindCreatures`,
  `crmain.cc:101-144`) walked in row-major (blockx inner, blocky outer) order.
- **Impact:** (a) over-collection cost per fan-out; (b) the documented tie-break ordering
  divergence (`monster_events.rs` GL#24) — creatures in one 64×64 chunk but different 16×16 blocks
  arrive in chunk-insertion order rather than 772 block order, affecting which monster reacts
  first to a move. Observable only in tie-break edge cases.
- **Fix:** add a dedicated 16×16 creature bucket index (separate from the 64×64 tile chunk, exactly
  as 772 splits them). Tile chunks can remain 64×64.

## 6. `blocks_sight` blocks on missing tiles — MED, SUSPECT — ✅ FIXED

- **Rust:** `map/mod.rs:85` `None => true` (a missing interior tile blocks sight).
- **1098:** `Map::checkSightLine` iterates `if (const Tile* tile = getTile(...))` — a null tile is
  skipped and does **not** block. Rare on a fully-loaded OTBM but a divergence when a line crosses
  an unmapped hole.
- **Fix (with #1):** decide the intended behavior and align `None` with 1098 (`false`) unless a
  deliberate 772 difference is confirmed against `map.cc`/`info.cc`.

## 7. Dual creature lists can desync — LOW, GAP

- `TileBody.creatures` (stack rendering) and `Chunk.creatures` (spatial query) are both
  hand-maintained. `register_creature_at`/`unregister_creature_at` keep them in sync, but any
  direct `grid.register_creature`/`unregister_creature` call bypasses the tile list.
- **Fix:** funnel all creature placement through the `*_at` methods and `debug_assert!` the two
  lists agree in tests.

## 8. `find_item_position` full-world scan — LOW, GAP

- `grid.rs::find_item_position` scans every chunk × 4096 slots. Acceptable for occasional house /
  teleport / auto-close checks; if it ever lands on a hot path add a reverse `ItemId → Position`
  index.

## 9. Coarse `is_walkable` + stub `query_add` — LOW, SUSPECT

- `is_walkable` (`map/mod.rs`) checks only `BLOCK_SOLID` + ground, ignoring creatures and
  `BLOCKPATH`. `Tile::query_add` (`tile.rs`) is a `true` stub. Real movement legality lives in
  `pathfinding.rs`, so these may be intentional coarse helpers — confirm no caller relies on them
  for authoritative queryAdd semantics, and doc/gate them accordingly.

---

## Phased implementation plan

Ordered by gameplay/protocol risk, then scale. Each phase is independently shippable and testable.
Follow `tfs-cpp-references.md`: every changed behavior cites its C++ source in the module header.

### Phase 1 — Correctness fixes (HIGH, small, isolated) — ✅ COMPLETE

Targets findings **#1, #2, #6**.

1. **`blocks_sight` flag fix (#1, #6).** ✅ Changed `map/mod.rs::blocks_sight` to test
   `body.flags & flags::UNTHROW != 0`; `None` branch now returns `false` (matching 1098
   `Map::isTileClear` null-tile behavior, `src/map.cpp:499-501`). Same wrong-flag bug fixed in
   `game_world_player_throw.rs::is_tile_clear_for_throw`, which cites the same C++ source.
2. **Wire stack cap (#2).** ✅ **Era correction:** the audit originally assumed both eras cap
   creatures at 10. Verified against C++ source — only **772** caps (`gameserver/src/protocolgame.cpp:572-574`,
   `if (++count == 10) return;`); **1098 does not** (`src/protocolgame.cpp:669-682` increments
   `count` but never checks it in the creature loop). Added `tile_description_caps_creatures()`
   to the `ProtocolCodec` trait — `false` for 1098, `true` for 772 — threaded through both
   `get_tile_description` and `count_tile_description` (must stay byte-identical for the
   `send_map_description_packet` debug assert).
3. **Tests:** ✅
   - `tests/map_los.rs` — added `sight_not_blocked_by_blockpath_only`,
     `sight_not_blocked_by_blocksolid_only`, `sight_blocked_by_unthrow`,
     `sight_not_blocked_by_missing_tile`; updated `map_with_wall` to use `UNTHROW`; flipped
     `throw_possible_ignores_solid_without_unthrow` assertion.
   - `tests/map_description.rs` — added `tile_description_772_caps_creatures_at_ten`,
     `tile_description_1098_does_not_cap_creatures`,
     `tile_description_772_shorter_than_1098_for_crowded_tile`.
4. **Verify:** ✅ `cargo test -p tfs-rust-core map_los` (11 passed), `cargo test -p tfs-rust-net
   map_description protocol_compat` (58 passed), full lib suites (465 + 26 passed), `cargo clippy`
   clean.

**Exit criteria:** ✅ monster kiting/flee LOS matches expectations over furniture; 772 tiles cap
at 10 things (1098 does not, matching C++); count/write passes stay byte-identical.

### Phase 2 — Storage invariants (MED, small) — ✅ COMPLETE

Targets findings **#3, #7**.

1. **Void-tile registration (#3).** ✅ `Map::register_creature_at` now detects when the target
   tile is absent and emits `tracing::error!` (release) + `debug_assert!` (debug/test) — never
   panics in release (per `tfs-packets.md` validation rules). `unregister_creature_at` mirrors
   with `tracing::warn!` (unregister-on-void is less harmful but still indicates untracked
   state). The old silent-drop path is gone: a void placement is now observable.
2. **Single placement path (#7).** ✅ `SparseGrid::register_creature` /
   `unregister_creature` narrowed from `pub` to `pub(super)` so only `map/mod.rs` (the `*_at`
   wrappers) can call them; the grid's own `#[cfg(test)] mod tests` retains child-module
   access. Added `SparseGrid::debug_assert_creature_lists_agree` (delegated through
   `Map::debug_assert_creature_lists_agree`) that verifies every `Chunk.creatures` entry is on
   some tile's `TileBody.creatures` list and vice versa — all `debug_assert!`-gated so the body
   compiles out in release.
3. **Harness invariant fix.** ✅ Test harnesses (`minimal_world`-based tests) were registering
   creatures at positions with no tile, violating the "creatures stand on valid tiles"
   invariant the audit makes explicit. Added `ensure_walkable_tile_if_absent` and wired it into
   the central `insert_monster_with_config` / `insert_monster_from_type` / `insert_npc` /
   `insert_spectator_player` / `player_walk` move paths. Does NOT overwrite intentionally-placed
   tiles. This fixed 10 pre-existing test harnesses that silently relied on the old
   silent-drop behavior.
4. **Tests:** ✅
   - `tests/map_storage.rs` — `register_creature_at_on_void_tile_is_noop` (debug panics,
     release logs+drops; creature absent from both lists), `unregister_creature_at_on_void_tile_is_noop`,
     `creature_lists_agree_after_batch_of_moves` (register/move/unregister batch + consistency check).
   - `map/grid.rs` `mod tests` — `debug_assert_catches_chunk_list_creature_not_on_tile`,
     `debug_assert_catches_tile_list_creature_not_in_chunk` (both `#[cfg(debug_assertions)]`,
     confirm the checker trips on each desync direction), `debug_assert_passes_on_clean_grid`.
5. **Verify:** ✅ `cargo test -p tfs-rust-core --lib` → 468 passed, 2 ignored (was 458+10
   failed); all 9 integration test binaries pass (map_storage 3, map_los 11, + 7 others);
   `cargo test -p tfs-rust-net` → 98 passed. `cargo clippy` — zero warnings in changed files
   (`map/mod.rs`, `map/grid.rs`, `tests/map_storage.rs`, `sim_harness.rs`); pre-existing
   sim_harness unused-import/variable warnings shifted line numbers only.

**Exit criteria:** ✅ no code path can desync the two creature lists (grid seam is `pub(super)`,
consistency checker catches both desync directions); void placement is observable (error log +
debug panic).

### Phase 3 — Player fan-out via the grid (HIGH for scale, medium effort)

Targets finding **#4**.

1. Add a `GameWorld` helper that resolves spectator **connections** through
   `grid.collect_spectators` (over the multi-floor Z span) → filter to creatures with a `ConnId`
   → `protocol_can_see`. Reuse the monster-side `spectator_z_range`.
2. Repoint `spectator_conns` and the `broadcast_*` family at the new helper. Keep the old
   full-scan behind a cfg/feature or delete once parity is proven.
3. **Tests:** equivalence test — for a seeded world, the grid-based spectator set equals the
   current full-scan set for a range of positions/floors (including underground ±2 and the
   surface/underground boundary).
4. **Verify:** run the full `tfs-rust-core` + `tfs-rust-net` suites; spot-check a load scenario if
   a bench harness exists.

**Exit criteria:** broadcasts produce byte-identical output to the full-scan path while touching
only local creatures; O(all players) removed from the per-event hot path.

### Phase 4 — 16×16 creature buckets for 772 parity (MED, larger)

Targets finding **#5** (and closes GL#24).

1. Introduce a creature spatial index at 16×16 granularity, separate from the 64×64 tile chunk
   (mirroring 772's `TFindCreatures` blocks vs sectors). Keep tile storage untouched.
2. Update `collect_spectators`/`collect_spatial_spectators` to iterate 16×16 blocks in the 772
   row-major order (blockx inner, blocky outer), removing the SlotMap-key sort fallback where the
   natural block order now matches.
3. **Tests:** port/extend the GL#24 tie-break scenario — assert move-stimulus fan-out order matches
   772 block order; assert `collect_spectators` no longer over-collects beyond the block set.
4. Cite `crmain.cc:101-144` `TFindCreatures::getNext`.

**Exit criteria:** fan-out set is tight to the viewport; multi-monster reaction order matches 772
in the previously-divergent tie-break cases.

### Phase 5 — Query ergonomics + cleanup (LOW, opportunistic)

Targets findings **#8, #9**.

1. Add a reverse `ItemId → Position` index **only if** profiling shows `find_item_position` on a
   hot path; otherwise document it as intentionally O(n) and cold.
2. Clarify/gate `is_walkable` and `Tile::query_add`: doc-comment that they are coarse helpers, or
   wire `query_add` to the real queryAdd rules and audit callers.

**Exit criteria:** no accidental reliance on stub/coarse helpers; item lookup cost understood.

---

## Reference index

| Concern | 772 (`reference/cipsoft-772/.../src/`) | 1098 (repo-root `src/`) |
|---------|----------------------------------------|-------------------------|
| Sector / tile storage | `map.hh` `TSector`, `map.cc` | — (OTBM in RAM) |
| Object priority chain | `map.hh` `PRIORITY_*`, `GetFirstObject` | `Tile::getThing` |
| Creature spatial search | `crmain.cc:101-144` `TFindCreatures` | `map.cpp Map::getSpectators` |
| Line of sight / throw | `info.cc` `ThrowPossible` (`UNTHROW`) | `map.cpp Map::checkSightLine` (`BLOCKPROJECTILE`) |
| Tile description wire | `gameserver/src/protocolgame.cpp` | `src/protocolgame.cpp GetTileDescription` |
