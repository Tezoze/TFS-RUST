# Sector refresh, decay, and map residency — parity plan (2026-09-06)

Audit of `crates/tfs-rust-core/src/sector_refresh.rs` and the map load path against the
`tibia-game-master` corpus (`map.cc`, `operate.cc`, `reader.cc`, `main.cc`, `time.cc`,
`objects.cc`) and the shipped runtime pack (`reference/cipsoft-772/runtime/dat/map.dat`,
`runtime/origmap/*.sec`, `runtime/map/*.sec`).

Companion: `docs/772_PARITY_GAP_AUDIT.md` §1.6 (Step 10 shipped the first cut of
`sector_refresh.rs`). This document supersedes §1.6's "DONE" for the items listed under
[Gaps](#3-gaps) and is the porting plan for Step 11.

---

## 0. Decisions (summary)

| Question | Decision |
|---|---|
| Lazy / proximity sector loading from OTBM? | **No.** Corpus loaded every `.sec` eagerly at boot (`LoadMap`); `GetMapContainer` never hits disk. Monsters/NPCs do sleep without players (`crnonpl.cc:2550`, `:1763`), but sleeping creatures still occupy tiles, `ProcessCreatures` / `ProcessSkills` touch every creature each second, monster homes respawn in empty areas, decay and refresh run map-wide — every populated chunk is referenced every second. Keep eager `SparseGrid`. |
| Mirror ORIGMAP `.sec` streaming on a reader thread? | **No.** Load-time snapshot of `TILEFLAG_REFRESH` tiles is the modern equivalent; identical outcome, no I/O, no async double-check. |
| Mirror `SwapSector` / `.swp`? | **No.** 2005 object-pool pressure; invisible to players. |
| Refresh cadence | `RefreshedCylinders = 8` per wall-clock minute (shipped `map.dat`), raster over the **full sector grid**, not only columns with snapshots. Value lives in `MechanicsProfile` / `772.lua`. |
| Post-refresh creature displacement | **Add** (`operate.cc:2832-2892`). |
| Recursive delete on refresh | **Add** — reuse `decay_apply.rs::destroy_item_tree`. |
| `refresh_map()` gate | **Add** `SectorRefreshable` per sector (corpus `RefreshMap` gates too). |
| Restore mechanism | Replace remove/add through the move pipeline with a raw `place_snapshot` (≙ `LoadObjects`): no destination chain, no stack merge, no Lua move events, one tile update. |
| Boot decay for map items | **Add** one post-load pass ≙ `LoadObjects` → `CronExpire(-1)`. |
| `RemainingExpireTime` from ORIGMAP | **Add** via TFS `ATTR_DURATION` on OTBM item nodes (patch script → parser → snapshot → `start_decay`). |
| Live-map persistence across reboot (dropped items, corpses, fields, remaining decay) | **Policy decision — deferred.** Corpus does it (`SaveMap`/`LoadMap`); TFS pristine-on-boot is what ships today. If adopted: delta file over OTBM, never `.sec`. |
| Map memory / boot time | Eager stays. Quality work, ordered: ground-as-`u16` instead of `Item`, dense chunk tiles, parallel chunk build, spawn placement profiling. Not part of Step 11. |

---

## 1. Corpus behaviour (source of truth)

### 1.1 Sector model

- `TSector { Object MapCon[32][32]; uint32 TimeStamp; uint8 Status; uint8 MapFlags; }` (`map.hh:74-79`).
- Grid `matrix3d<TSector*>`; bounds from `map.dat`: X 996–1043, Y 984–1031, Z 0–15 (48×48 XY columns). Defaults before `map.dat` are 1000–1015 and `RefreshedCylinders = 1` (`map.cc:345-351`) — **the `1` is a fallback, not the shipped value**.
- `MapFlags & 1` = sector contains at least one `Refresh` tile; per-tile `Attributes[3] |= 0x100`.
- `InitMap` → `LoadMap` loads every `MAPPATH/*.sec` at boot (`map.cc:1769`). `GetMapContainer` returns `NONE` for a missing sector, never loads (`map.cc:2318-2338`).
- `SwapSector`/`UnswapSector`: LRU by `TimeStamp` when `GetFreeObjectSlot` fails; `Object::exists` auto-unswaps (`map.cc:47-58`). Memory only.

### 1.2 Refresh pipeline

| Stage | Corpus | Detail |
|---|---|---|
| Minute job | `RefreshCylinders` (`operate.cc:2964-2988`), called when `RoundNr >= NextMinute` (`main.cc:383`) | Static `RefreshX/RefreshY` raster over `SectorXMin..=Max × SectorYMin..=Max`; **`RefreshedCylinders` columns per call (shipped = 8)**; every Z per column. Full sweep = 2304 / 8 = 288 min. |
| Gate | `SectorRefreshable` (`operate.cc:2796-2821`) | `TFindCreatures` ±31 around sector center, `FIND_PLAYERS`; skip if any player `CanSeeFloor(z)` (`cr.hh:576-582`: z≤7 sees ≤7, else \|Δz\|≤2). Checked at enqueue and again at reply-apply (async reader thread). |
| Data | `LoadSectorOrder` → `ProcessLoadSectorOrder` (`reader.cc:59-67`) | Reader thread parses **ORIGMAP** `.sec`, emits only `Refresh` tiles' `Content=`. Applied on game thread via `ProcessReaderThreadReplies(RefreshSector, …)` every round (`main.cc:357`). |
| Tile restore | `map.cc::RefreshSector` (`:1307-1345`) | Requires `Sec->MapFlags & 1`. Per tile: `DeleteObject` every non-creature object (recursive; `CronStop` on `EXPIRE`, `map.cc:1876`), then `LoadObjects(Stream, Con)` → fresh objects, `CronExpire(-1)` full `TotalExpireTime`, then `RemainingExpireTime=N` → `CronChange`. Creatures stay. |
| Creature post-pass | `operate.cc::RefreshSector` (`:2832-2892`) | `TFindCreatures` ±16 around center, `FIND_ALL`, same Z. If the creature's field now has a non-creature `UNPASS` object → `SearchFreeField(&x,&y,&z, 1, 0, false)`; on failure NPC → `startx/y/z`, monster → `delete`, player → `error(...)` and left. Then `Move(0, CrObject, MapCon, -1, false, NONE)`. |
| Full refresh | `RefreshMap` (`operate.cc:2895`) | Reboot save window only. Iterates all sectors, **also gated by `SectorRefreshable`**, parses ORIGMAP synchronously, calls the `map.cc` variant (no creature post-pass). |

Only `Refresh`-flagged tiles are touched; other tiles in the same sector keep drops/corpses.

### 1.3 Decay (cron) across events

Round-based min-heap `vector<TCronEntry>` keyed by ObjectID, deadline `RoundNr + Delay`
(`map.cc:35`, `:209-231`, `:270-284`; `RoundNr` = seconds since boot, `time.cc:3-6`). Never
recomputes elapsed time — every event drops or creates an entry:

| Event | `EXPIRE` objects (corpse / field / pool / lit torch) | Dropped non-`EXPIRE` items |
|---|---|---|
| Boot `LoadMap` | `AppendObject` → `ChangeObject` → `CronExpire(-1)` = full `TotalExpireTime`; `RemainingExpireTime=N` in the `.sec` → `CronChange(Obj, N)`. Offline time **not** subtracted. No "clear fields/corpses" pass. | Loaded as-is from last `MAPPATH` save. |
| Refresh | `DeleteObject` → `CronStop`; ORIGMAP restore → fresh full timer (ORIGMAP pools may carry `RemainingExpireTime=5`). | **Deleted** on `Refresh` tiles. |
| Swap | Entries untouched; `ProcessCronSystem` → `getObjectType()` unswaps. | Raw bytes to `.swp`. |
| `SaveSector` (`map.cc:1095-1112`) | All non-creatures written; `REMAININGEXPIRETIME` from `CronInfo(Obj)`. | Written as-is (`Amount`, `Charges`, …). Only filter: `isCreatureContainer`. |

Expiry (`ProcessCronSystem`, `operate.cc:2763-2794`): container → `Empty(Obj, Remainder = target Capacity)`; `Change(Obj, ExpireTarget, 0)`; target type 0 → `Delete`.

Instance attribute keywords (`objects.cc:235-253`): `Content`, `Amount`, `Charges`, `String`,
`PoolLiquidType`, `Responsible`, `RemainingExpireTime`, `SavedExpireTime`, `RemainingUses`, …

---

## 2. Rust today

### 2.1 Map residency (`map/grid.rs`, `map/mod.rs`)

- `SparseGrid = FxHashMap<ChunkKey, Box<Chunk>>`; `Chunk` = 64×64 on one floor, `Box<[Option<Box<Tile>>; 4096]>` + `SmallVec<[CreatureId; 4]>` creature list. `SECTOR_SIZE = 16` for `TFindCreatures` order.
- Eager: `Map::from_map_data` builds every tile; every OTBM item (including ground) becomes an `Item` in the SlotMap.
- Boot log (2026-09-05): `map_chunks=3355 map_tiles=7,848,617 tile_stack_item_refs=752,021 items_slotmap=8,565,851`; OTBM + `items.xml` merge ≈ 8.5 s; spawn placement ≈ 17 s; total ≈ 37 s.
- No decay is started for map items at load (`start_decay` is never called from `map/mod.rs`; only from inventory load and live mutations).
- OTBM parser reads `OTBM_ATTR_TILE_FLAGS` and `OTBM_ATTR_ITEM` at tile level; item nodes carry count/charges. No `ATTR_DURATION`.

### 2.2 `sector_refresh.rs`

Matches corpus:
- Snapshot on `TILEFLAG_REFRESH` (`1<<5` → runtime `1<<27`), houses excluded. OTBM flag set aligned to ORIGMAP via `scripts/patch_otbm_refresh_from_origmap.py` — **694,625 / 694,625** after the 2026-09-06 token-parser fix (the first run under-counted at 673,561 and cleared 20,861 correct flags; lesson 438).
- `sector_refreshable`: ±31 box, `can_see_floor`.
- Minute cadence via `GetRoundForNextMinute` (`game_world_tick.rs:59`).
- Restore: remove non-creature items (`cancel_item_decay` ≙ `CronStop`), re-add snapshot (`internal_add_item_to_tile` → `start_decay` ≙ `CronExpire(-1)`). Creatures stay.
- `refresh_map()` reserved for Lua `Game.refreshMap()` / reboot.
- `DecayClockModel::RoundNumber` in profile ≙ round-based cron.

---

## 3. Gaps

Ordered by observable impact.

| # | Gap | Corpus | Rust | Effect |
|---|---|---|---|---|
| G1 | Cylinders per minute | 8 (`map.dat`) | 1, hardcoded | 8× slower sweep |
| G2 | Raster domain | Full 48×48 grid incl. empty columns | Only XYs with snapshots (`RefreshCylinderState::ensure_index`) | Sweep period = \|xys\| min instead of 288 min; with 673k tiles likely ~30 h. Drops linger 6–7× longer |
| G3 | Creature post-pass | `operate.cc:2832-2892` | Missing | Monsters/NPCs left inside restored `UNPASS` objects |
| G4 | Recursive delete | `DeleteObject` deletes container contents | `internal_remove_item_from_tile(…, u16::MAX)` → `detach` + `items.remove(id)` | Player-dropped bags on refresh tiles orphan children in `items` / `container_registry`; viewers not auto-closed |
| G5 | `refresh_map` gate | `SectorRefreshable` per sector | Unconditional | Reboot-time refresh wipes tiles under players |
| G6 | Boot decay for map items | `LoadObjects` → `CronExpire` full time | Never started | OTBM fields / pools / lit torches frozen until first refresh sweep re-creates them (hours) |
| G7 | `RemainingExpireTime` | ORIGMAP `RemainingExpireTime=N` → `CronChange` | Lost in OTBM conversion and snapshot | Seeded pools restore with full duration instead of N s |
| G8 | Restore via move pipeline | `LoadObjects` is raw creation | `internal_add_item_to_tile`: `query_destination_chain`, stack merge, Lua move events, per-item broadcast | Safe today only by ordering (everything removed first, teleport re-added last). Fragile; extra script side effects |
| G9 | Snapshot content parity | ORIGMAP `Content=` | OTBM tile content; patch script aligned **flags** and inserted content only for the 2,576 missing tiles | Existing OTBM refresh tiles whose stack diverges from ORIGMAP restore the wrong items — unverified |
| G10 | Snapshot decay state | n/a | `RefreshItemSnap` clones `ItemAttributes` wholesale | If G6 starts decay before snapshotting, clones carry `DecayState::True` and `start_decay` early-returns on restore |
| G11 | Live-map persistence | `SaveMap` / `LoadMap` keep drops, corpses, fields, remaining decay across reboot (non-Refresh tiles) | OTBM read-only; pristine on boot | Largest observable divergence in this area. **Policy decision**, see §5 |

---

## 4. Plan — Step 11

**Architecture:** all work in focused modules. `GameWorld` gets thin delegates only.
`sector_refresh.rs` is ~320 lines; the creature post-pass and raw placement go in the same file
(single concern) unless it passes ~800 lines, in which case split `sector_refresh/{raster,place,creatures}.rs`.
Boot decay goes in a new `map_decay_init.rs` (not `map/mod.rs`, not `GameWorld`).

### 4.1 Profile literal

- `MechanicsProfile.refreshed_cylinders: u16` (default `8`); `772.lua` key `refreshedCylinders = 8`.
- Cite `map.dat` `RefreshedCylinders = 8` and `map.cc:351` fallback in the doc comment.

### 4.2 Sector-keyed snapshots + iterator raster (G1, G2)

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct SectorKey { sx: u16, sy: u16, z: u8 }   // ORIGMAP 32×32

pub struct SectorBounds { sx: RangeInclusive<u16>, sy: RangeInclusive<u16>, z: RangeInclusive<u8> }

pub struct RefreshSnapshots {
    bounds: SectorBounds,                                   // derived from OTBM tile extents at load
    sectors: HashMap<SectorKey, Box<[(Position, TileRefreshSnap)]>>,
}

struct SectorRaster { bounds: SectorBounds, cursor: (u16, u16) }   // Iterator<Item=(u16,u16)>, cycles
```

- `Map.refresh_snapshots: HashMap<Position, _>` → `Map.refresh: RefreshSnapshots` (build in `from_map_data`; `Box<[_]>` because immutable after load; same for `down` / `top` in `TileRefreshSnap`).
- `refresh_cylinders`: `for (sx, sy) in raster.by_ref().take(profile.refreshed_cylinders)`; for each Z in bounds: `sector_refreshable` → `sectors.get(&key)` → restore. Advancing 8 **grid** columns per minute (including empty ones) reproduces the 288-minute sweep exactly; the lookup makes empty columns free.
- Replace `RefreshCylinderState` with `SectorRaster` on `GameWorld` (`refresh_raster`).

### 4.3 Raw placement (G4, G8, G10)

`fn place_snapshot(&mut self, pos: Position, snap: &TileRefreshSnap)` in `sector_refresh.rs`:

1. Collect non-creature `ItemId`s on the tile (ground, down, top).
2. Destroy each with the shared recursive helper — promote `decay_apply.rs::destroy_item_tree` to `pub(crate)` (or move to a small `item_destroy.rs` and call from both). It already clears `container_registry`, auto-closes viewers, cancels decay, removes from the SlotMap. Detach from the tile first (`detach_item_from_tile` for flag recompute, or a batch clear + one `reset_item_tile_flags` pass).
3. Build new `Item`s from the snapshot; set `duration` from `RefreshItemSnap.remaining_ms` when present (§4.5); strip `decaying` state in `RefreshItemSnap::from_item`.
4. Write `ground_item` / `ground` / `down_items` / `top_items` directly on `TileBody`; recompute tile flags once.
5. `start_decay` on each placed item (≙ `CronExpire(-1)` then `CronChange`).
6. One spectator tile update (`UpdateTile`-shaped), not per-item add/remove. Gate guarantees no viewer within draw range, so this is traffic hygiene, not parity.

No `query_destination_chain`, no `fire_item_move_events`, no stack merge.

### 4.4 Creature post-pass (G3)

After all Z-sectors of a column are restored, per refreshed `(sx, sy, z)`:

- Query creatures in ±16 box around the sector center on that Z via the chunk spatial index (`SparseGrid` `chunk.creatures`), not a `conn_to_creature` scan.
- If the creature's tile holds a non-creature item whose type is `UNPASS` (`BLOCK_SOLID`):
  - existing free-field search, radius 1, `!Ignore` semantics of `SearchFreeField(…, 1, 0, false)`;
  - failure: NPC → spawn/start position; monster → remove (`delete Creature`); player → `tracing::warn!` and leave.
  - success → internal relocate (the corpus `Move(0, …, -1, false, NONE)` — no walk, no path).
- Also use `chunk.creatures` for `sector_refreshable` (±31, players only, `CanSeeFloor`).

### 4.5 Decay parity (G6, G7)

**Boot pass — `map_decay_init.rs`:** after `Map::from_map_data` and **after** `RefreshSnapshots` is built, iterate `items` whose type `can_decay`, call `start_decay` (full `decay_time`; item `duration` if set). Cite `map.cc:860-907` (`LoadObjects` → `AppendObject` → `CronExpire(-1)` → `RemainingExpireTime` → `CronChange`).

**`RemainingExpireTime` → OTBM:**
- `scripts/patch_otbm_refresh_from_origmap.py`: emit TFS `ATTR_DURATION` (`item.h`, id 16, `u32` ms) on inserted item nodes when ORIGMAP has `RemainingExpireTime=N` (N s → N·1000 ms). Re-run over `forgotten.otbm` for the ~26 affected tiles.
- `tfs-rust-content/src/otbm.rs`: parse `ATTR_DURATION` on item nodes into `TileThing`/item props.
- `RefreshItemSnap { …, remaining_ms: Option<u32> }` populated from the loaded item's `duration`.
- `map/mod.rs::tile_from_data` sets `duration` on the created `Item`; the boot pass then starts decay with it.

### 4.6 `refresh_map` gate (G5)

`refresh_map()` iterates `sectors` keys, applies `sector_refreshable(sx, sy, z, …)` before restoring, mirrors `RefreshMap` (`operate.cc:2895-2900`). No creature post-pass on this path (corpus calls the `map.cc` variant).

### 4.7 Content verification (G9)

One-off script (`/tmp` or `scripts/verify_otbm_refresh_content.py`): for every ORIGMAP `Refresh` tile, map `Content=` TypeIDs → server ids (`convert_itemid_to_clientid.build_server_to_client`), compare ordered stack against the OTBM tile (ground + items, `Amount` / `Charges`). Report mismatches by category (missing item, extra item, count differs, order differs). Decide per category whether to patch the OTBM or accept. Until this runs, G9 is the only remaining "restores the wrong thing" risk.

### 4.8 Tests

`cargo test -p tfs-rust-core --lib sector_refresh` plus:

- raster: 8 columns per call over mocked bounds; wraps X then Y; empty columns advance the cursor without work
- `place_snapshot`: dropped container with children → no orphans in `items` / `container_registry`; open viewer auto-closed
- restored `EXPIRE` item gets `DecayState::True` with full duration; with `remaining_ms` gets that duration
- monster on restored `UNPASS` tile relocated (radius 1) / removed when boxed in; NPC returns to spawn; player untouched
- `refresh_map` skips a sector with a visible player; restores when player is at z=11 vs floor 7
- boot pass: OTBM field on a non-refresh tile decays after `decay_time` without any refresh
- snapshot taken before boot decay: clone has `DecayState::False`

### 4.9 Verification

```
rtk cargo check -p tfs-rust-core -p tfs-rust-content
rtk cargo clippy -p tfs-rust-core -p tfs-rust-content
rtk cargo test -p tfs-rust-core --lib sector_refresh
rtk cargo test -p tfs-rust-core --lib decay_apply
rtk cargo test -p tfs-rust-core --lib map_decay_init
python3 scripts/patch_otbm_refresh_from_origmap.py --dry-run
```

Then update `docs/772_PARITY_GAP_AUDIT.md` §1.6 (cadence 8/min over full grid; creature post-pass; boot decay) and add a `tasks/lessons.md` entry: *`RefreshedCylinders` default in `map.cc` is a pre-`map.dat` fallback; shipped pack literal is 8; raster runs over the whole grid, not populated columns.*

---

## 4A. Implementation plan — ordered phases

Each phase is one PR-sized change, independently `cargo test`-green, in dependency order.
Sub-agent split per `TFS-subagents.mdc`: one `generalPurpose` agent per phase for code, one
`shell` agent for verify; parent integrates and owns `tasks/todo.md` / `tasks/lessons.md`.

Existing helpers to reuse (do not re-implement):

| Need | Reuse |
|---|---|
| Recursive item destroy | `decay_apply.rs::destroy_item_tree` (`:403`) → promote to `pub(crate)` |
| Free field search r=1 | `spawn_placement.rs::search_free_field(center, 1)` (`info.cc:761`, east-first spiral, rejects houses/occupied/UNPASS) |
| Creature box query | `map/grid.rs::collect_spectators_sector_order(cx, cy, z, rx, ry, &mut out)` (16×16 sector order) |
| Floor visibility | `idle_stimulus.rs::creature_can_see_floor` — move to a shared place or call from `sector_refresh.rs`; delete the private `can_see_floor` copy |
| Tile flag recompute | `map/mod.rs::reset_item_tile_flags` / `tile_remaining_props` |
| Full tile resend | `tfs_rust_net::map_description::send_update_tile` (0x69) via a new `broadcast_tile_refresh` in `game_world_spectators.rs` |
| Creature removal | `game_world_lifecycle.rs::remove_creature` |
| NPC start position | NPC `base`/runtime start pos (as used by `npc/host.rs::set_start_position`) |
| Monster start | `Monster.spawn_position` |

### Phase 0 — content verification (no Rust)

`scripts/verify_otbm_refresh_content.py` (may live in `/tmp` if not worth keeping):

1. Reuse `patch_otbm_refresh_from_origmap.py` parsers (`load_origmap_refresh`, `load_otbm`, `parse_tile_attrs`, `load_client_to_server`).
2. For each ORIGMAP `Refresh` tile: ordered `[server_id, amount?, charges?, remaining_expire?]` vs OTBM tile `[ground, item nodes…]`.
3. Categorise: `missing`, `extra`, `count_diff`, `order_diff`, `ok`. Print totals and first 50 examples per category.
4. Decision per category recorded in this doc before Phase 3 (patch OTBM vs accept).

Exit criterion: mismatch report exists; `ok` share known.

**Result (2026-09-06, token-based parse, after flag fix):** flags 694,625 / 694,625, zero divergence either way. Content on those tiles: `ok` 661,017 (98.1%) · `type_diff` 5,090 · `count_diff` 4,073 · `order_diff` 3,351 · `otbm_extra` 28 · `otbm_missing` 2 (+ the 21,064 newly flagged tiles not yet content-compared). Observed causes: `count_diff` mostly OTBM `count=1` vs ORIGMAP no `Amount` (equivalent) with a real subset (e.g. `2027` count 6); `type_diff` looks like client→server remap collisions under the "first server id wins" rule (`4346`↔`919`, `4630`↔`4624`); `otbm_missing` are ORIGMAP type `1`/`3` non-visible objects; `order_diff` needs the corpus chain order checked before deciding. Per-category decision still pending.

### Phase 1 — profile literal + sector-keyed snapshots + raster (G1, G2)

Files: `formulas.rs`, `data/formulas/772.lua`, `sector_refresh.rs`, `map/mod.rs`, `game_world.rs` (field rename only), `game_world_tick.rs` (no change expected).

1. `formulas.rs`: `pub refreshed_cylinders: u16` on `MechanicsProfile`; default `8` in the 772 profile literal block (~`:621`), `8` in the 1098 block too (same corpus); parser `int_or(&formulas, "refreshedCylinders", 8)` near `decayClock` (~`:1029`). `772.lua`: `refreshedCylinders = 8, -- map.dat RefreshedCylinders; map.cc:351 fallback is 1`.
2. `sector_refresh.rs`:
   ```rust
   pub(crate) struct SectorKey { pub sx: u16, pub sy: u16, pub z: u8 }
   pub(crate) struct SectorBounds { pub sx: RangeInclusive<u16>, pub sy: RangeInclusive<u16>, pub z: RangeInclusive<u8> }
   pub struct RefreshSnapshots { bounds: SectorBounds, sectors: HashMap<SectorKey, Box<[(Position, TileRefreshSnap)]>> }
   pub(crate) struct SectorRaster { bounds: SectorBounds, cursor: Option<(u16, u16)> }   // Iterator, cycles; None = before first
   ```
   - `RefreshSnapshots::build(iter: impl Iterator<Item = (Position, TileRefreshSnap)>) -> Self` — groups by key, computes bounds from min/max of **all map tiles** (pass extents from `from_map_data`), converts `Vec` → `Box<[_]>`.
   - `RefreshSnapshots::sector(&self, key) -> Option<&[(Position, TileRefreshSnap)]>`, `::keys()`, `::bounds()`.
   - `SectorRaster::next` — corpus order: X inner, Y outer, wrap both (`operate.cc:2970-2979`).
   - `TileRefreshSnap { ground: Option<RefreshItemSnap>, down: Box<[RefreshItemSnap]>, top: Box<[RefreshItemSnap]> }`.
3. `map/mod.rs`: `pub refresh: RefreshSnapshots` replaces `refresh_snapshots: HashMap<Position, _>`; `from_map_data` collects into a `Vec` then `RefreshSnapshots::build`. Update the ~15 test constructors (`refresh_snapshots: HashMap::new()` → `refresh: RefreshSnapshots::default()`).
4. `GameWorld.refresh_cylinder_state` → `refresh_raster: SectorRaster` (initialised from `map.refresh.bounds()` after map build).
5. `refresh_cylinders`:
   ```rust
   let n = self.mechanics.profile.refreshed_cylinders as usize;
   let cols: Vec<(u16, u16)> = self.refresh_raster.by_ref().take(n).collect();
   for (sx, sy) in cols { for z in bounds.z.clone() {
       if !self.sector_has_snapshot(sx, sy, z) { continue; }        // free skip for empty columns
       if !self.sector_refreshable(sx, sy, z) { continue; }
       self.refresh_sector(SectorKey { sx, sy, z });                  // Phase 2 replaces body
   } }
   ```
   Return count of refreshed tiles for logging.

Tests: raster advances exactly `n` grid columns per call and wraps; empty column consumes a slot with no work; two populated sectors in one column both restore in the same minute; existing three tests keep passing (update to new API).

### Phase 2 — raw placement + recursive destroy (G4, G8, G10)

Files: `sector_refresh.rs`, `decay_apply.rs` (visibility only), `game_world_spectators.rs` (+1 fn).

1. `decay_apply.rs`: `fn destroy_item_tree` → `pub(crate)`. No behaviour change.
2. `RefreshItemSnap::from_item`: copy `item_type`, `count`, `attributes` but clear decay state / duration on the clone (`set_decaying(DecayState::False)`, `set_duration(0)`), add `remaining_ms: Option<u32>` (filled in Phase 4; `None` for now).
3. `sector_refresh.rs::refresh_sector(&mut self, key)`:
   ```rust
   let tiles = self.map.refresh.sector(key)?.to_vec();      // clone once; sector-sized, not map-sized
   for (pos, snap) in tiles { self.place_snapshot(pos, &snap); }
   ```
4. `place_snapshot(&mut self, pos, snap)`:
   1. `old: Vec<ItemId>` = ground + down + top from `TileBody`.
   2. For each: `detach_item_from_tile(pos, id)` **without** the per-item broadcast — factor the existing body into `detach_item_from_tile_silent` used by both, or add a `notify: bool` parameter; then `destroy_item_tree(id)`.
   3. Build items: `self.items.insert(snap.to_item())` for ground, down, top in that order; write `body.ground_item`, `body.ground`, `body.down_items`, `body.top_items` directly; call `reset_item_tile_flags`-equivalent full recompute once.
   4. `start_decay(id)` for each new item (Phase 4 seeds `duration` first).
   5. `broadcast_tile_refresh(pos)` — new in `game_world_spectators.rs`: collect spectators in draw range, `send_update_tile` per viewer. Given the ±31 gate this is normally zero viewers; keep it for `refresh_map()` and Lua.
   No `internal_add_item_to_tile`, no `query_destination_chain`, no `fire_item_move_events`.
5. Delete `refresh_one_tile`.

Tests: dropped backpack containing 3 items → after refresh, none of the 4 ids exist in `items`, `container_registry` has no entry, an open viewer got a close; restored `EXPIRE` type is `DecayState::True` with full duration; snapshot clone taken from a decaying item has `DecayState::False`; teleport tile snapshot restores without redirecting items (previously only safe by ordering).

### Phase 3 — creature post-pass + `refresh_map` gate (G3, G5)

Files: `sector_refresh.rs` (or `sector_refresh/creatures.rs` if the file passes ~800 lines).

1. `sector_refreshable(sx, sy, z)`: replace the `conn_to_creature` Vec with `collect_spectators_sector_order(cx, cy, z_scan, 31, 31, &mut buf)` over the Z set that can see `z` (reuse `idle_acquire_search_z_range`-style helper), filter players, `creature_can_see_floor(p.z, z)`.
2. After `refresh_sector(key)` in `refresh_cylinders` (not in `refresh_map`): `displace_blocked_creatures(key)`:
   ```rust
   let cx = sx * 32 + 16; let cy = sy * 32 + 16;
   collect_spectators_sector_order(cx, cy, z, 16, 16, &mut buf);   // FIND_ALL, same Z (operate.cc:2833-2836)
   for cid in buf {
       if !tile_has_unpass_item(pos) { continue; }                    // non-creature UNPASS on creature's tile
       match self.search_free_field(pos, 1) {
           Some(dest) => self.internal_teleport_creature(cid, dest),   // corpus Move(…, -1, false, NONE): no walk
           None => match kind {
               Npc     => self.internal_teleport_creature(cid, npc_start_pos),
               Monster => self.remove_creature(cid),
               Player  => tracing::warn!(?pos, "player affected by refresh"),
           },
       }
   }
   ```
   Use whichever internal relocate the port already has for `Move(0, CrObject, MapCon, -1, false, NONE)` (the summon/teleport path), not the walk pipeline.
3. `refresh_map()`: iterate `map.refresh.keys()`, apply `sector_refreshable` per key, then `refresh_sector`. Return tile count. No post-pass (corpus `RefreshMap` calls the `map.cc` variant).

Tests: monster on a restored `UNPASS` stack moves to an adjacent free tile; boxed-in monster is removed; boxed-in NPC returns to start; player is untouched and a warn is emitted; `refresh_map` skips the sector with a player at z=7 and restores with the player at z=11.

### Phase 4 — decay parity (G6, G7)

Files: new `map_decay_init.rs`, `lib.rs` (mod), `run_server.rs` + `sim_harness.rs` (one call each), `tfs-rust-content/src/otbm.rs`, `map/mod.rs::tile_from_data`, `scripts/patch_otbm_refresh_from_origmap.py`, `sector_refresh.rs` (`remaining_ms`).

1. `map_decay_init.rs`:
   ```rust
   //! Boot decay for map items — corpus `LoadObjects` → `AppendObject` → `CronExpire(-1)` →
   //! `RemainingExpireTime` → `CronChange` (`map.cc:860-907`, `:270-284`).
   impl GameWorld {
       pub(crate) fn start_map_item_decay(&mut self) -> usize {
           let ids: Vec<ItemId> = self.items.iter().filter(|(_, it)| it.parent_is_tile()).map(|(id, _)| id).collect();
           ids.into_iter().filter(|&id| self.can_decay(id)).inspect(|&id| self.start_decay(id)).count()
       }
   }
   ```
   Call once in `run_server.rs` after `GameWorld` is built and **after** `map.refresh` exists (snapshots must predate decay), before spawns. Same in `sim_harness.rs` world builders. Log the count next to the "GameWorld ready" line.
2. `otbm.rs`: parse `OTBM_ATTR_DURATION` (TFS `item.h` `ATTR_DURATION`, id 16, `u32` ms) on item-node props → `TileThing::ItemNodeProps` already carries raw props; extend the item-node attribute decoder that reads count/charges to also return `duration_ms: Option<u32>`.
3. `map/mod.rs::tile_from_data`: `item.set_duration(ms)` when present.
4. `RefreshItemSnap::from_item`: `remaining_ms = item.duration_raw_ms()` when non-zero (this is the OTBM-seeded value, since snapshots are taken before Phase 4's boot pass); `to_item` sets `duration` from it; `place_snapshot` then `start_decay` → `CronChange`-equivalent.
5. `patch_otbm_refresh_from_origmap.py`: `SecItem.remaining_expire: int | None`; parse `RemainingExpireTime=N`; `encode_item_node` emits `OTBM_ATTR_DURATION = 16` + `u32(N*1000)`. Re-run on `forgotten.otbm`; expect ~26 tiles touched.

Tests: OTBM field on a non-refresh tile decays after `decay_time` with no refresh involved; ORIGMAP-seeded pool with `duration = 5000` expires in 5 rounds after restore; boot pass count equals number of decaying map items in a small fixture; snapshot taken before boot pass has `DecayState::False`.

### Phase 5 — docs, audit, lesson

1. `docs/772_PARITY_GAP_AUDIT.md` §1.6: cadence 8/min over full grid; creature post-pass; raw placement; boot decay; `ATTR_DURATION`.
2. `tasks/lessons.md`: *`RefreshedCylinders` in `map.cc:351` is the pre-`map.dat` fallback; shipped pack is 8 and the raster covers the whole sector grid, not populated columns. `TFindCreatures` filters XY only — never add a Z check unless the corpus caller does.*
3. `tasks/todo.md` Step 11 items ticked.

### Sequencing and risk

| Phase | Depends on | Risk | Mitigation |
|---|---|---|---|
| 0 | — | None (read-only) | — |
| 1 | — | ~15 test constructors touch `refresh_snapshots` | `RefreshSnapshots::default()`; mechanical |
| 2 | 1 | `detach_item_from_tile` split; tile flag recompute correctness | Reuse existing flag helpers; test teleport/UNPASS/blocking flags after restore |
| 3 | 2 | Internal relocate path choice | Use the summon/teleport relocate already used by `CreateMonster` |
| 4 | 2 | Boot pass cost (8.5M items scanned once) | Single linear pass, ~ms; log count. `ATTR_DURATION` parse must tolerate absence |
| 5 | 1–4 | — | — |

Phase 1 alone fixes the two highest-impact gaps and can ship independently.

### Verification per phase

```
rtk cargo check -p tfs-rust-core -p tfs-rust-content
rtk cargo clippy -p tfs-rust-core -p tfs-rust-content -- -D warnings
rtk cargo test -p tfs-rust-core --lib sector_refresh
rtk cargo test -p tfs-rust-core --lib decay_apply          # phases 2, 4
rtk cargo test -p tfs-rust-core --lib map_decay_init       # phase 4
rtk cargo test -p tfs-rust-core --test map_storage --test map_los --test inventory_container_gaps   # phase 1 constructors
python3 scripts/patch_otbm_refresh_from_origmap.py --dry-run   # phase 4
```

Live check after Phase 1: run the server, watch the minute log line for refreshed-tile counts; a full sweep should complete in ≈ (sector columns / 8) minutes.

---

## 5. Deferred — live-map persistence (G11)

Corpus persists the entire live map (`SaveMap` on reboot / shutdown; `LoadMap` at boot), so
dropped items, corpses, fields and their remaining decay survive a restart on non-`Refresh` tiles.
TFS ships pristine-on-boot (only house tiles via `IOMapSerialize`). This shard currently follows TFS.

If corpus behaviour is wanted:

- New module `map_delta.rs`. Track a dirty-tile set (any non-house tile whose stack was mutated). On shutdown (and optionally periodic), serialise `(Position, ordered stack: type, count, attributes, remaining decay ms)` for dirty tiles only. Format: compact binary or RON; **not** `.sec`, **not** a second OTBM.
- Boot: load OTBM → apply deltas (replace stack) → build refresh snapshots → boot decay pass (§4.5). Offline time is not subtracted (corpus parity).
- Refresh tiles: a delta on a `Refresh` tile is legitimate (corpus saves them too); the first sweep wipes it.
- Reboot-time `RefreshMap` before save (`main.cc:428`) is already available as `refresh_map()`.

Do not start this without an explicit go-ahead: it changes what players find after a restart.

---

## 6. Deferred — map memory / boot time

Eager residency stays. Same outputs, better Rust; not Step 11:

1. **Ground as `u16`, not `Item`.** 7.85M of 8.57M SlotMap items are ground; `TileBody` already caches `ground: Option<u16>` next to `ground_item: Option<ItemId>`. Materialise an `Item` only when an `ItemId` is required (transform, Lua userdata, move). Largest memory lever; cuts across move/use/Lua — planned-refactor sized.
2. **Dense chunk tiles.** `Box<[Option<Box<Tile>>; 4096]>` → chunk-local `Vec<Tile>` + `[u16; 4096]` slot index (`u16::MAX` = empty). One fewer allocation per tile; better locality for 64×64 spectator / path walks.
3. **Parallel chunk build.** mmap / `Bytes` OTBM; build chunks per floor with `rayon` before `GameWorld` exists (pure CPU, no game state). SlotMap insertion stays serial — another reason (1) matters.
4. **Spawn placement** (~17 s of the 37 s boot) — profile before touching the map loader.

Measure RSS with `ps` after the "GameWorld ready" log line before prioritising any of these.

---

## 7. Explicit non-goals

- Related NPC nit (not refresh): `npc_players_in_sleep_range` filters `z == npc.z`; corpus `TFindCreatures::getNext` (`crmain.cc:101-144`) filters XY only. A player one floor up within 10×10 keeps a corpus NPC awake. Track separately.
- ORIGMAP `.sec` reader thread or `.swp` swap emulation.
- TFS 1.4.2 `Map::refreshMap` every minute (froze the game thread at 673k tiles).
- TVP `TILESTATE_REFRESH = 1<<11` (collides with this port's `TELEPORT`); runtime flag stays `1<<27`.
