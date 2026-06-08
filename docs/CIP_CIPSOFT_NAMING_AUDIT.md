# Cip / CipSoft naming audit

Inventory for a **legal/rebrand rename** of identifiers and user-facing strings that contain `cip`, `Cip`, or `CipSoft`. Re-audited 2026-06-08 (previous pass: 2026-06-01).

**Scope:** Tracked project sources (`crates/`, `data/`, `docs/`, `scripts/`, `tools/`, `.cursor/`, `.agents/`, `tasks/`). Excludes ignored trees (`gameserver/`, `tibia-game-master/`, repo-root `src/` TFS C++ mirror, `reference/classic-772/`) unless noted.

**Re-verify after edits:**

```bash
rg -i 'cipsoft|\bcip\b' crates data scripts tools docs tasks .agents .cursor \
  --glob '!reference/**' --glob '!log/**'
rg 'CipSoft|cipsoft_|CIPSOFT_|StepSpeedModel::LinearGo|CipSoftPoly|Cipsoft' crates data
rg -l 'cipsoft|CipSoft' crates scripts tools --glob '*.{rs,py,sh}'
```

**Counts (2026-06-08):** ~683 lines in scoped dirs above; ~271 in `crates/**/*.rs`; ~175 symbol hits in `crates/` + `data/`.

---

## Summary

| Category | Approx. count | Rename priority |
|----------|---------------|-----------------|
| **Public Rust modules** | 1 | High |
| **Public Rust functions** | 7 | High |
| **Public Rust structs** | 1 | High |
| **Public Rust constants** | 3 | High |
| **Public enum variants** | 2 | High |
| **Lua parser aliases** (`"cipsoft"`, `"cip"`, `"poly"`, `"shortway"`) | 4 arms | High (shard configs) |
| **Lua config string literals (shipped)** | 0 active vendor keys | Lower — `772.lua` now uses `"772"` / `"reverse"` |
| **Env / shell identifiers** | 5+ | Medium (ops docs) |
| **Files named `*cipsoft*`** | 3 | High |
| **Rust test function names** | ~18 | Medium |
| **Private Rust helpers** | 4 | Medium (rename with Tier 1) |
| **Code references to Tier 1 symbols** | ~80+ uses across 10+ files | High (follow rename) |
| **Comment/doc mentions only** | ~170+ lines in 32 `.rs` files; ~280+ in 12 `.md` | Lower (review published docs) |
| **Scripts / tools (dev-only)** | 8 files | Medium |

**Change since 2026-06-01:** Pathfinding parity and OTB waypoint overlay work added a **`tfs-rust-content` module**, **pathfinding public API**, **CIPSOFT_* constants**, and several **772 parity docs**. Shipped `772.lua` no longer sets `levelExp = "cipsoft"` or `stepSpeedModel = "cipsoft"` (defaults come from Rust `MechanicsProfile::for_version(772)`).

---

## Locked vocabulary (2026-06-08)

**Policy:** Public Rust names describe **formula shape or behavior**, not vendor or era. Era selection stays in `MechanicsProfile` / `clientVersion`. Config-facing strings may use `"772"` where already established (`PlayerSpeedModel::Classic772`, `formulas.playerSpeed = "772"`).

**Do not use:** `Era772`, `CipSoft772`, or `*_772` on core logic functions (project naming rule). **Exception:** config/profile enum variants like `Classic772` (already shipped).

### Walk / step timing

| Old | **New (locked)** |
|-----|------------------|
| `StepSpeedModel::LinearGo` | `StepSpeedModel::LinearGo` |
| `linear_go_effective_speed` | `linear_go_effective_speed` |
| `cipsoft_speed_from_profile` | `linear_go_speed_from_profile` |
| `cipsoft_step_duration_ms` | `linear_go_step_duration_ms` |
| Lua `stepSpeedModel`: `"cipsoft"` / `"cip"` | **`"linearGo"`** (+ deprecated aliases) |

Runtime flag **`beat_driven_loop`** — keep (already neutral).

### Pathfinding / reverse terrain path

| Old | **New (locked)** |
|-----|------------------|
| `uses_reverse_terrain_path` | `uses_reverse_terrain_path` |
| `cipsoft_effective_waypoints` | `effective_terrain_waypoints` |
| `scan_cipsoft_min_waypoints` | `scan_min_terrain_waypoints` |
| `cipsoft_shortway_heuristic` | `reverse_path_heuristic` |
| `REVERSE_PATH_VIEW_RADIUS` | `REVERSE_PATH_VIEW_RADIUS` |
| `DEFAULT_TERRAIN_WAYPOINTS` | `DEFAULT_TERRAIN_WAYPOINTS` |
| `CHASE_PATH_MAX_STEPS` | `CHASE_PATH_MAX_STEPS` |
| `CIPSOFT_MAX_CLOSED_NODES` | `REVERSE_PATH_MAX_CLOSED_NODES` |
| `CIPSOFT_NEIGHBOR_OFFSETS` | `REVERSE_PATH_NEIGHBOR_OFFSETS` |

**Keep as-is:** `PathSearchModel::Reverse`, Lua `"reverse"` / `"shortway"`, debug `log_shortway` (internal).

### Level exp

| Old | **New (locked)** |
|-----|------------------|
| `LevelExpModel::DeltaPoly` | `LevelExpModel::DeltaPoly` |
| Lua `levelExp`: `"cipsoft"` | **`"delta"`** / `"poly"` (+ deprecated `"cipsoft"`) |

### Content / OTB overlay

| Old | **New (locked)** |
|-----|------------------|
| `objects_srv` (mod + file) | `objects_srv` |
| `CipsoftGroundWaypoints` | `ObjectsSrvGroundWaypoints` |
| `audit_cipsoft_waypoints.rs` | `audit_objects_srv_waypoints.rs` |
| `TFS_CIPSOFT_OBJECTS_SRV` | `TFS_OBJECTS_SRV` |
| `TFS_CIPSOFT_772_DIR` | `TFS_REFERENCE_772_DIR` |
| Shell `CIPSOFT_*` | `REF_772_*` |
| `reference/classic-772/` | `reference/classic-772/` (Phase 6 — path + scripts) |

### Prose (docs, comments, user-facing strings)

| Old | **New (locked)** |
|-----|------------------|
| “772” | “772 mechanics”, “beat-driven profile” |
| “reference stack” | “reference stack”, “tibia-game-master stack” |
| Planned `monster_beat_driven_*` | `monster_beat_driven_*` |

### Do not rename

| Item | Reason |
|------|--------|
| `tibia1.cipsoft.com`, `tibia2.cipsoft.com` in `patch_tibia_client.py` | Literal bytes inside leaked 7.72 client EXEs |
| `cr*.cc` in internal `//!` traceability comments | Decompile filenames — counsel decides redaction for public docs |

---

## Implementation phases (execution order)

Each phase ends with `cargo check`, `cargo clippy`, and targeted `cargo test` for touched crates. Do **not** mix unrelated phases in one PR unless noted.

### Phase 1 — Walk / step timing (`tfs-rust-core`)

**Goal:** Remove vendor names from the hottest path (walk timers, beat loop gate).

| Step | Work |
|------|------|
| 1.1 | Rename `StepSpeedModel::LinearGo` → `LinearGo`; default in `MechanicsProfile::for_version(772)` |
| 1.2 | Rename `linear_go_effective_speed` → `linear_go_effective_speed` |
| 1.3 | Rename private helpers in `walk/walk_timing.rs` |
| 1.4 | Update call sites: `walk/mod.rs`, `walk_timing.rs`, `creature/vocation.rs`, `run_server.rs`, `game_loop.rs` |
| 1.5 | Rename Phase 1 tests in `formulas.rs`, `walk/mod.rs`, `game_loop.rs` |
| 1.6 | Grep: `StepSpeedModel::LinearGo`, `linear_go_effective_speed` → zero in `crates/` |

**Files:** `formulas.rs`, `walk/walk_timing.rs`, `walk/mod.rs`, `creature/vocation.rs`, `run_server.rs`, `game_loop.rs`, `tests/mechanics_formulas.rs`.

### Phase 2 — Pathfinding (`tfs-rust-core`)

**Goal:** Neutral reverse-terrain path vocabulary; depends on Phase 1 only for unrelated grep cleanliness.

| Step | Work |
|------|------|
| 2.1 | Rename public fns + private heuristic + all `CIPSOFT_*` constants |
| 2.2 | Update `monster_ai.rs` imports and call sites |
| 2.3 | Update `chase_debug.rs` if any stale references |
| 2.4 | Rename pathfinding tests (~7 fns) + `monster_ai` test `test_772_allow_diagonal_true_stays_*` |
| 2.5 | Grep: `cipsoft_`, `CIPSOFT_` → zero in `pathfinding.rs`, `monster_ai.rs` |

**Files:** `pathfinding.rs`, `monster_ai.rs`, parity doc cross-refs deferred to Phase 7.

### Phase 3 — Level exp enum (`tfs-rust-core`)

**Goal:** Small, can land same PR as Phase 1 if preferred.

| Step | Work |
|------|------|
| 3.1 | Rename `LevelExpModel::DeltaPoly` → `DeltaPoly` |
| 3.2 | Update `combat/math.rs` match arm + doc refs |
| 3.3 | Rename combat tests; update `formulas.rs` defaults test |

**Files:** `formulas.rs`, `combat/math.rs`, `tests/mechanics_formulas.rs`.

### Phase 4 — Content crate (`tfs-rust-content`)

**Goal:** Module/file rename + struct; keep neutral fns unchanged.

| Step | Work |
|------|------|
| 4.1 | Rename `objects_srv.rs` → `objects_srv.rs`; `pub mod objects_srv` |
| 4.2 | Rename `CipsoftGroundWaypoints` → `ObjectsSrvGroundWaypoints` |
| 4.3 | Update `otb_patch.rs`, `items.rs`, `pipeline.rs` if referenced |
| 4.4 | Rename test file + `audit_cipsoft_waypoints_vs_otb` → `audit_objects_srv_waypoints_vs_otb` |
| 4.5 | Rename `scripts/audit_cipsoft_waypoints_vs_otb.py` → `audit_objects_srv_waypoints_vs_otb.py` |
| 4.6 | `cargo test -p tfs-rust-content` |

**Files:** `objects_srv.rs`, `lib.rs`, `otb_patch.rs`, `items.rs`, audit test, audit script.

### Phase 5 — Lua parser + config compatibility

**Goal:** New canonical config strings; old vendor strings still parse (deprecated).

| Step | Work |
|------|------|
| 5.1 | Add parser arms: `"linearGo"` → `LinearGo`, `"delta"` → `DeltaPoly` |
| 5.2 | Keep `"cipsoft"`, `"cip"`, `"poly"`, `"shortway"` as deprecated aliases (comment + optional `tracing::warn` once at load) |
| 5.3 | Neutralize comments in `data/formulas/772.lua` and `1098.lua` |
| 5.4 | Document migration in `docs/PROTOCOL_VERSIONING.md` §formulas |

**Files:** `formulas.rs`, `data/formulas/*.lua`.

### Phase 6 — Env vars, shell, reference paths

**Goal:** Ops-facing renames with dual-read period.

| Step | Work |
|------|------|
| 6.1 | `TFS_OBJECTS_SRV` (read old `TFS_CIPSOFT_OBJECTS_SRV` if unset) |
| 6.2 | `TFS_REFERENCE_772_DIR` (read old `TFS_CIPSOFT_772_DIR`) |
| 6.3 | Shell `REF_772_*` replacing `CIPSOFT_*` in `reference_paths.sh`, `tibia_game_*.sh` |
| 6.4 | Path `reference/classic-772/` → `reference/classic-772/` in scripts, tests, `.gitignore`, `reference/README.md` |
| 6.5 | `.tibia-cipsoft/` state dir → `.tibia-ref-772/` (or keep symlink note in docs) |

**Files:** `objects_srv.rs` → `objects_srv.rs`, `scripts/lib/reference_paths.sh`, `scripts/tibia_game_*.sh`, `.gitignore`, test constants.

### Phase 7 — Docs, rules, comments

**Goal:** User-facing prose neutral; internal `cract.cc` cites OK per counsel.

| Step | Work |
|------|------|
| 7.1 | Sweep `docs/` parity docs (highest hit count first) |
| 7.2 | Update `.agents/rules/*.md` |
| 7.3 | Rust `//!` / `///` comments: replace “CipSoft” with “772 mechanics” or formula name |
| 7.4 | Update `tasks/todo.md`, `tasks/lessons.md` |
| 7.5 | Refresh this audit doc counts; re-run verify commands at top |

**Defer heavy doc edits** until Phases 1–4 land so symbol names in docs match code.

### Phase 8 — Dev scripts & tools

**Goal:** Parity/compare tooling; not in production binary.

| Step | Work |
|------|------|
| 8.1 | `tools/simulate_idle_todo_compare.py` — rename `CipSoftCreature`, `linear_go_effective_speed` |
| 8.2 | `scripts/compare_chase_pathfinding.py` — JSON key `"cipsoft"` → `"reference"` |
| 8.3 | `scripts/compare_chase_live_logs.py` — `--cip` → `--ref` |
| 8.4 | `scripts/patch_otb_waypoints.sh` comment |
| 8.5 | **Leave** `tibia*.cipsoft.com` literals in `patch_tibia_client.py` |

### Phase summary

| Phase | Scope | PR suggestion |
|-------|--------|---------------|
| **1** | Walk / `LinearGo` | Single PR |
| **2** | Pathfinding constants + fns | Single PR (after 1) |
| **3** | `DeltaPoly` | Same PR as 1 or tiny follow-up |
| **4** | `objects_srv` content crate | Single PR |
| **5** | Lua parser aliases | Single PR (after 1–3) |
| **6** | Env / paths / shell | Single PR (may need local symlink for devs) |
| **7** | Docs + comments | One or more PRs |
| **8** | Scripts/tools | Optional cleanup PR |

**Verification gate (all phases):**

```bash
rg 'CipSoft|cipsoft_|CIPSOFT_|Cipsoft' crates data scripts tools
# Target: zero in crates/ after Phase 4; zero everywhere except patch_tibia_client.py hostnames after Phase 8
cargo check --workspace && cargo test --workspace
```

---

## Tier 1 — Public Rust API (inventory → locked names)

### `tfs-rust-core` — formulas & walk

| Current name | Kind | File | **New name** |
|--------------|------|------|--------------|
| `linear_go_effective_speed` | `pub fn` | `formulas.rs` | `linear_go_effective_speed` |
| `StepSpeedModel::LinearGo` | enum variant | `formulas.rs` | `StepSpeedModel::LinearGo` |
| `LevelExpModel::DeltaPoly` | enum variant | `formulas.rs` | `LevelExpModel::DeltaPoly` |

**Call sites:** `formulas.rs`, `walk/mod.rs`, `walk/walk_timing.rs`, `creature/vocation.rs`, `run_server.rs`, `game_loop.rs`, `tests/mechanics_formulas.rs`, `combat/math.rs`.

### `tfs-rust-core` — pathfinding

| Current name | Kind | File | **New name** |
|--------------|------|------|--------------|
| `cipsoft_effective_waypoints` | `pub fn` | `pathfinding.rs` | `effective_terrain_waypoints` |
| `scan_cipsoft_min_waypoints` | `pub fn` | `pathfinding.rs` | `scan_min_terrain_waypoints` |
| `uses_reverse_terrain_path` | `pub fn` | `pathfinding.rs` | `uses_reverse_terrain_path` |
| `REVERSE_PATH_VIEW_RADIUS` | `pub const` | `pathfinding.rs` | `REVERSE_PATH_VIEW_RADIUS` |
| `DEFAULT_TERRAIN_WAYPOINTS` | `pub const` | `pathfinding.rs` | `DEFAULT_TERRAIN_WAYPOINTS` |
| `CHASE_PATH_MAX_STEPS` | `pub const` | `pathfinding.rs` | `CHASE_PATH_MAX_STEPS` |
| `CIPSOFT_MAX_CLOSED_NODES` | `const` (private) | `pathfinding.rs` | `REVERSE_PATH_MAX_CLOSED_NODES` |
| `CIPSOFT_NEIGHBOR_OFFSETS` | `const` (private) | `pathfinding.rs` | `REVERSE_PATH_NEIGHBOR_OFFSETS` |

**Call sites:** `pathfinding.rs`, `monster_ai.rs`, parity docs.

### `tfs-rust-content` — OTB waypoint overlay

| Current name | Kind | File | **New name** |
|--------------|------|------|--------------|
| `objects_srv` | `pub mod` | `lib.rs` | `objects_srv` |
| `CipsoftGroundWaypoints` | `pub struct` | `objects_srv.rs` | `ObjectsSrvGroundWaypoints` |
| `resolve_objects_srv_path` | `pub fn` | `objects_srv.rs` | *(unchanged)* |
| `parse_walkable_waypoints` | `pub fn` | `objects_srv.rs` | *(unchanged)* |
| `apply_waypoints_to_item_speeds` | `pub fn` | `objects_srv.rs` | *(unchanged)* |
| `resolve_server_id_for_patch` | `pub fn` | `objects_srv.rs` | *(unchanged)* |
| `overlay_otb_speeds_from_objects_srv` | `pub fn` | `objects_srv.rs` | *(unchanged)* |

**Path strings (Phase 6):** `reference/classic-772/runtime/dat/objects.srv`, env `TFS_OBJECTS_SRV`.

---

## Tier 1b — Files named `*cipsoft*`

| Path | Role | **New path (Phase 4/6)** |
|------|------|--------------------------|
| `crates/tfs-rust-content/src/objects_srv.rs` | `objects.srv` parser + OTB overlay | `objects_srv.rs` |
| `crates/tfs-rust-content/tests/audit_cipsoft_waypoints.rs` | Integration audit test | `audit_objects_srv_waypoints.rs` |
| `scripts/audit_cipsoft_waypoints_vs_otb.py` | Standalone audit script | `audit_objects_srv_waypoints_vs_otb.py` |

Rename module/file together with `pub mod objects_srv` (Phase 4).

---

## Tier 2 — Private Rust helpers (Phase 1–2)

| Current name | File | **New name** |
|--------------|------|--------------|
| `cipsoft_speed_from_profile` | `walk/walk_timing.rs` | `linear_go_speed_from_profile` |
| `cipsoft_step_duration_ms` | `walk/walk_timing.rs` | `linear_go_step_duration_ms` |
| `cipsoft_shortway_heuristic` | `pathfinding.rs` | `reverse_path_heuristic` |

---

## Tier 3 — Rust test functions (Phases 1–4)

| Current test name | File | **New name (pattern)** |
|-------------------|------|------------------------|
| `linear_go_effective_speed_matches_gameserver` | `formulas.rs` | `linear_go_effective_speed_matches_gameserver` |
| `defaults_772_match_cipsoft` | `formulas.rs` | `defaults_772_match_linear_go_profile` |
| `cipsoft_step_duration_matches_notify_go` | `walk/mod.rs` | `linear_go_step_duration_matches_notify_go` |
| `cipsoft_diagonal_step_duration_quantizes_waypoints_before_beat` | `walk/mod.rs` | `linear_go_diagonal_step_duration_quantizes_waypoints_before_beat` |
| `shipped_772_formulas_match_cipsoft_defaults` | `tests/mechanics_formulas.rs` | `shipped_772_formulas_match_profile_defaults` |
| `fight_mode_modifiers_match_cipsoft_integer_shape` | `combat/math.rs` | `fight_mode_modifiers_match_772_integer_shape` |
| `probe_value_matches_cipsoft_formula_bounds` | `combat/math.rs` | `probe_value_matches_classic_formula_bounds` |
| `cipsoft_neighbor_order_matches_expand_loop` | `pathfinding.rs` | `reverse_path_neighbor_order_matches_expand_loop` |
| `cipsoft_effective_waypoints_defaults_missing_to_150` | `pathfinding.rs` | `effective_terrain_waypoints_defaults_missing_to_150` |
| `scan_cipsoft_min_waypoints_ignores_blocked_tiles` | `pathfinding.rs` | `scan_min_terrain_waypoints_ignores_blocked_tiles` |
| `uses_reverse_terrain_path_matches_772_profile` | `pathfinding.rs` | `uses_reverse_terrain_path_matches_772_profile` |
| `reverse_with_allow_diagonal_still_uses_cipsoft_expansion` | `pathfinding.rs` | `reverse_with_allow_diagonal_still_uses_reverse_expansion` |
| `cipsoft_heuristic_prefers_toward_origin` | `pathfinding.rs` | `reverse_path_heuristic_prefers_toward_origin` |
| `beat_driven_loop_flag_follows_cipsoft_profile` | `game_loop.rs` | `beat_driven_loop_flag_follows_linear_go_profile` |
| `counters_fire_at_cipsoft_thresholds` | `subsystem_counters_772.rs` | `counters_fire_at_beat_driven_thresholds` |
| `test_772_allow_diagonal_true_stays_cipsoft_path_stack` | `monster_ai.rs` | `test_772_allow_diagonal_true_stays_reverse_path_stack` |
| `ground_tile_speeds_match_cipsoft_waypoint_expectations` | `items.rs` | `ground_tile_speeds_match_objects_srv_waypoint_expectations` |
| `audit_cipsoft_waypoints_vs_otb` | `audit_cipsoft_waypoints.rs` | `audit_objects_srv_waypoints_vs_otb` |

---

## Tier 4 — Lua / config strings (Phase 5)

### Shipped `data/formulas/772.lua`

| Location | Value | Notes |
|----------|-------|--------|
| Comments only | “reverse TShortway”, etc. | Neutralize in Phase 5 |
| `playerSpeed = "772"` | Neutral | Keep |
| No `levelExp` / `stepSpeedModel` keys | — | Defaults from `MechanicsProfile::for_version(772)` |

### Parser — canonical vs deprecated (Phase 5)

| Key | **Canonical** | Deprecated aliases (keep parsing) | Maps to |
|-----|---------------|-----------------------------------|---------|
| `stepSpeedModel` | `"linearGo"` | `"cipsoft"`, `"cip"` | `StepSpeedModel::LinearGo` |
| `pathSearch` | `"reverse"` | `"shortway"`, `"cipsoft"` | `PathSearchModel::Reverse` |
| `levelExp` | `"delta"`, `"poly"` | `"cipsoft"` | `LevelExpModel::DeltaPoly` |

**Backward compatibility:** Deprecated aliases remain for at least one release; optional one-time `tracing::warn` when a deprecated string is loaded.

### Other Lua

| Location | Value |
|----------|-------|
| `data/formulas/1098.lua` | Comment mentions “classic CipSoft linear speed” |

---

## Tier 5 — Env vars, shell, and reference paths (Phase 6)

| Current | **New (locked)** | File(s) | Notes |
|---------|------------------|---------|--------|
| `TFS_CIPSOFT_OBJECTS_SRV` | `TFS_OBJECTS_SRV` | `objects_srv.rs` | Dual-read old name |
| `TFS_CIPSOFT_772_DIR` | `TFS_REFERENCE_772_DIR` | `reference_paths.sh` | Dual-read old name |
| `CIPSOFT_RUNTIME` | `REF_772_RUNTIME` | `reference_paths.sh` | Shell-only |
| `CIPSOFT_CLIENT` | `REF_772_CLIENT` | `reference_paths.sh` | Shell-only |
| `CIPSOFT_STATE` | `REF_772_STATE` | `reference_paths.sh`, `tibia_game_online.sh` | Shell-only |
| `reference/classic-772/` | `reference/classic-772/` | scripts, tests, `.gitignore` | Gitignored tree |
| `.tibia-cipsoft/` | `.tibia-ref-772/` | `.gitignore` | Runtime state |
| `tibia1.cipsoft.com`, `tibia2.cipsoft.com` | *(unchanged)* | `patch_tibia_client.py` | Literal client bytes |

---

## Tier 6 — Rust comments only (Phase 7)

Per-file match counts in `crates/**/*.rs` (2026-06-08). Replace “CipSoft” with **“772 mechanics”** or the locked formula name (`LinearGo`, reverse terrain path, etc.); keep `cract.cc` file refs in internal blocks if counsel allows.

| File | ~Matches |
|------|----------|
| `pathfinding.rs` | 58 |
| `formulas.rs` | 42 |
| `walk/walk_timing.rs` | 24 |
| `monster_ai.rs` | 22 |
| `combat/math.rs` | 17 |
| `objects_srv.rs` | 13 |
| `walk/mod.rs` | 9 |
| `audit_cipsoft_waypoints.rs` (test) | 8 |
| `subsystem_counters_772.rs` | 7 |
| `game_loop.rs` | 6 |
| `game_world.rs` | 4 |
| `otb_patch.rs` | 4 |
| `creature/vocation.rs`, `creature/base.rs`, `creature_todo.rs`, `items.rs`, `idle_stimulus.rs` | 3 each |
| `mechanics_formulas.rs`, `monster_targets.rs`, `game_world_tick.rs` | 2 each |
| 12 other core files | 1 each |

Typical pattern: `// C++ reference: …` or `cract.cc` citations.

---

## Tier 7 — Documentation (Phase 7)

| File | ~Matches | Notes |
|------|----------|--------|
| `docs/TFS-RUST_772_Monster_AI_Dance_Chase_Parity.md` | 77 | **New** — heavy vendor prose |
| `docs/PROTOCOL_VERSIONING.md` | 53 | Architecture |
| `docs/TFS-RUST_772_Chase_Path_Parity_Gaps.md` | 33 | **New** |
| `docs/TIBIA_GAME_MASTER_DEV.md` | 24 | Reference stack setup |
| `docs/PROTOCOL_VERSIONING_IMPLEMENTATION_PLAN.md` | 18 | Phase B mechanics |
| `docs/CODEBASE_AUDIT.md` | 16 | |
| `docs/TFS-RUST_772_Pathfinding_Creature_AI_Analysis.md` | 15 | **New** |
| `docs/GAME_LOOP_ARCHITECTURE.md` | 11 | |
| `docs/IDLE_STIMULUS.md` | 8 | |
| `tasks/lessons.md` | 8 | |
| `tasks/todo.md` | 7 | |
| `reference/README.md` | 7 | Ignored tree pointer |
| `docs/DIAGNOSTIC_FINDINGS_772_MONSTER_AI.md` | 2 | |
| `.agents/rules/tfs-core.md` | 4 | |
| `.agents/rules/mechanics.md`, `protocol-versioning.md` | 2 each | |
| `.cursor/rules/*.mdc` | 0 | Neutralized since prior audit |

Also `.gitignore` comments: “772 decompile”, `reference/classic-772/`.

---

## Tier 8 — Scripts & tools (Phase 8)

| File | ~Matches | Phase 8 renames |
|------|----------|-----------------|
| `scripts/tibia_game_dev.sh` | 13 | Prose + `REF_772_*` paths |
| `tools/simulate_idle_todo_compare.py` | 11 | `ReferenceCreature`, `linear_go_effective_speed` |
| `scripts/compare_chase_pathfinding.py` | 9 | JSON key `"reference"` |
| `scripts/compare_chase_live_logs.py` | 6 | `--ref` flag |
| `scripts/patch_tibia_client.py` | 6 | Paths only; **keep** hostname literals |
| `scripts/tibia_game_online.sh` | 4 | “reference stack” message |
| `scripts/audit_cipsoft_waypoints_vs_otb.py` | 5 | Rename file (Phase 4) + prose |
| `scripts/patch_otb_waypoints.sh` | 1 | Comment |
| `scripts/lib/reference_paths.sh` | 3 | `REF_772_*` vars |

Not part of production server binary; still review if scripts ship externally.

---

## Tier 9 — Related paths (not `cip*` Rust identifiers)

| Path / term | In repo |
|-------------|---------|
| `reference/classic-772/` | Gitignored; scripts + tests default here |
| `tibia-game-master/` | Ignited via above; 772 mechanics reference |
| `cr*.cc` citations in comments | Decompile filenames — retain for internal traceability or redact in public docs |

---

## What was excluded from this audit

- **False positives** for substring `cip` inside unrelated words (e.g. `cipher`, `recipient`) — none found in Rust identifiers.
- **Ignored C++ trees** (`gameserver/`, `src/`, `tibia-game-master/`, `reference/classic-772/`) — vendor strings in upstream/leaked assets; not part of Rust crate API.
- **`.cursor/debug-*.log`**, `log/*.old` — session logs, not source.

---

## Changelog

| Date | Action |
|------|--------|
| 2026-06-01 | Initial audit (post walk-speed fix) |
| 2026-06-08 | Re-audit: pathfinding public API, `tfs-rust-content` module/files, OTB waypoint tooling, expanded 772 parity docs; `772.lua` no longer ships `levelExp`/`stepSpeedModel` vendor keys; `.cursor` rules neutralized |
| 2026-06-08 | Locked vocabulary (`LinearGo`, `DeltaPoly`, reverse-terrain path, `objects_srv`) + 8-phase implementation order |
| 2026-06-08 | Phases 1–5 implemented in `crates/` + `data/formulas/`; Phases 6–8: `classic-772` paths, shell `REF_772_*`, docs/scripts/tools sweep |
