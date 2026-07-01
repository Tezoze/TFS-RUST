# TFS-RUST Refactor Audit

**Date:** 2026-06-28
**Scope:** Structural health of the Rust workspace — module organization, file/function size,
the `GameWorld` god-object, naming-rule compliance, and test/production separation.
**Out of scope:** Behavioral/parity correctness (see `docs/CODEBASE_AUDIT.md`).

---

## Executive Summary

The port is functionally rich but is accumulating **structural debt** concentrated in
`tfs-rust-core`, which is **52,629 LOC — 76% of all Rust in the workspace**. The pain is
not spread evenly: a handful of files and one god-object (`GameWorld`) account for most of
the mess.

| Crate | LOC | Health |
|-------|-----|--------|
| `tfs-rust-core` | 52,629 | **Needs work** — god-object + 2 mega-files |
| `tfs-rust-net` | 8,591 | OK — well-split codec/encode |
| `tfs-rust-content` | 5,906 | OK |
| `tfs-rust-common` | 2,013 | OK |
| `tfs-rust-db` | 1,915 | OK |
| `tfs-rust-lua` | 1,698 | OK |

**Top 5 refactor priorities (highest impact first):**

1. **Split `idle_stimulus.rs` (6,158 LOC) and `monster_ai.rs` (4,757 LOC).**
2. **Tame the `GameWorld` god-object** — `impl GameWorld` is spread across **33 files**.
3. **Move simulation/debug harness code out of the production `core` library.**
4. **Fix `_772` naming-rule violations on core functions** (147 occurrences).
5. **Decompose oversized functions** (20+ functions over 120 lines, longest is 317).

---

## 1. Mega-files: two files dominate

| File | LOC | Prod LOC | Test LOC | `fn` count |
|------|-----|----------|----------|-----------|
| `idle_stimulus.rs` | 6,158 | ~2,415 | ~3,743 (61%) | 145 |
| `monster_ai.rs` | 4,757 | ~2,835 | ~1,922 (40%) | 105 |
| `sim_harness.rs` | 2,437 | ~1,782 | ~655 | — |
| `pathfinding.rs` | 2,261 | ~1,228 | ~1,033 | — |
| `walk/mod.rs` | 2,081 | — | — | 46 |

### Findings

- `idle_stimulus.rs` is a **single 243 KB file** holding 145 functions plus ~3,700 lines of
  inline tests. It is effectively a sub-system, not a module. The IDE chokes on it and
  reviewers cannot reason about it.
- Both mega-files mix several concerns: idle/chase stepping, spell impact application,
  walk-branch execution, and target selection all live together.

### Recommendation

- Carve `idle_stimulus.rs` into a `monster_idle/` directory: e.g. `chase.rs`, `walk_branch.rs`,
  `spell_impact.rs`, `summon.rs`, plus `mod.rs` for the dispatch surface.
- Carve `monster_ai.rs` similarly (`onthink.rs`, `follow.rs`, `look.rs`, `move_planning.rs`).
- **Move inline `#[cfg(test)]` blocks to `tests/` sibling files or `#[path]` test modules.**
  Pulling ~5,600 test lines out of these two files alone roughly halves their size and makes
  the production logic legible.

---

## 2. `GameWorld` god-object

### Finding

`GameWorld` has **35 public fields** and its `impl` blocks are scattered across **33 source
files**. Method counts per file (inside `impl GameWorld`):

```
145  idle_stimulus.rs        35  game_world_inventory.rs    25  container_ui.rs
105  monster_ai.rs           34  player_inventory_query_add 24  monster_push.rs
 46  walk/mod.rs             30  game_world_spectators.rs   23  creature_todo.rs
                             28  spawn_lifecycle.rs         22  monster_targets.rs
```

This is the central structural problem: nearly every subsystem reaches into the same struct
via `&mut self`, which:
- forces `std::mem::take` / borrow-splitting gymnastics (the hygiene rule already calls this
  out — see `.cursor/rules/TFS-code-hygiene.mdc`),
- makes it impossible to reason about which subsystem owns which field,
- defeats the SlotMap-ID design intent of decoupling subsystems.

### Recommendation

This is a long-term effort — do **not** attempt in one pass. Incrementally:
- Group the 35 fields into cohesive sub-structs (e.g. `combat`, `spawns`, `containers`,
  `connections`) so methods borrow only what they need.
- Prefer **free functions that take the minimal `&mut` slices** over more `impl GameWorld`
  methods, per the entity-storage rule ("Pass IDs, not borrowed data").
- Treat "no new `impl GameWorld` file" as a soft rule going forward.

---

## 3. Simulation/debug harness shipped in the production library

### Finding

`crates/tfs-rust-core/src/lib.rs` compiles these into the production library:

- `pub mod sim_harness;` (2,437 LOC, 81 KB)
- `mod chase_debug;` (538 LOC)
- `mod sim_glibc_rand;` (9.9 KB)
- `mod test_world;` (`#[cfg(test)]` — OK)
- bin targets `chase_kite_sim.rs` (888 LOC) and `path_compare.rs`

`sim_harness` and `chase_debug` are test/diagnostic scaffolding but are part of the shipped
`core` API surface (`sim_harness` is even `pub`).

### Recommendation

- Move `sim_harness`, `chase_debug`, and `sim_glibc_rand` behind a `sim` cargo **feature**
  (`#[cfg(feature = "sim")]`), or into a separate `tfs-rust-sim` crate / `tests/` support
  module. This shrinks the production build, clarifies the public API, and stops diagnostic
  code from drifting into game logic.

---

## 4. Naming-rule violations: `_772` suffixes on core functions

### Finding

There are **147** `_772`-suffixed identifiers in `tfs-rust-core/src`. Many are on core game
functions, which directly violates the project's own always-on rule
(`TFS-Core` → *"No version suffix on core functions — era is config + profile, not function name"*):

```
monster_on_chase_noway_772        process_connections_772
monster_move_possible_planning_772 tick_ambiente_light_772
process_creatures_772             advance_beat_772
monster_exhausted_wait_772        clear_todo_772
monster_idle_summon_lifecycle_772 monster_can_kick_boxes_772
```

(Config enum variants like `Classic772` and `*_reads_772` tests are explicitly allowed.)

### Recommendation

- Rename core functions to behavior-based names and select era via `MechanicsProfile` /
  `beat_driven_loop`, e.g. `advance_beat_772` → `advance_beat`, `process_creatures_772` →
  `process_creatures_beat`. Where a function genuinely only runs in the beat-driven loop,
  name it for that behavior (`beat_*`) rather than the version number.

---

## 5. Oversized functions

### Finding

20+ production functions exceed 120 lines. Worst offenders:

| Lines | Location |
|-------|----------|
| 317 | `monster_ai.rs` `monster_do_attacking` |
| 287 | `walk/mod.rs` `on_walk` |
| 207 | `login_out.rs` `enqueue_initial_login_packets_1098` |
| 206 | `walk/mod.rs` `internal_move_creature_step` |
| 191 | `monster_ai.rs` `go_to_follow_creature` |
| 187 | `idle_stimulus.rs` `monster_idle_apply_spell_impact` |
| 180 | `pathfinding.rs` `path_matching_reverse` |

### Recommendation

- Extract cohesive blocks into named helpers (the hygiene rule's "extract before duplication"
  guidance). `monster_do_attacking` and `on_walk` in particular read as multi-stage state
  machines that would benefit from per-stage helpers.

---

## 6. Module fragmentation (counter-trend)

While the mega-files are too big, the opposite smell also exists: subsystems split across many
thin sibling files glued only by a shared prefix:

- `game_world_*` — 9 files (`game_world.rs`, `_inventory`, `_item_cylinder`, `_item_move`,
  `_lifecycle`, `_player`, `_player_throw`, `_save`, `_script`, `_spectators`, `_tick`)
- `player_inventory_*` — 5 files
- `monster_*` — 6 files

This is *flat* fragmentation: the relationships aren't visible in the tree. Prefer real
submodule directories (`game_world/`, `monster/`, `player/inventory/`) with a `mod.rs` that
documents the surface, rather than dozens of `prefix_suffix.rs` files at the crate root.

---

## 7. Lower-priority / housekeeping

- **`.unwrap()` / `.expect()`** — 283 in core, but **almost entirely in test code**
  (idle_stimulus: 0 in prod / 82 in tests; monster_ai: 7 in prod). The rule bans them in
  *production*; the 7 prod cases in `monster_ai.rs` plus the handful in `config.rs` /
  `pathfinding.rs` should be converted to `?` / `ok_or_else`. Test usage is acceptable but
  noisy.
- **Stray backup files** at repo root: `AGENTS.md.bak`, `config.lua.bak` (and `config.lua.dist`).
  Remove `.bak` files; they should not be tracked.
- **Vendored C++ reference** (`reference/` = 1.7 GB, `src/` = 2.8 MB) is large but expected as
  the porting spec — ensure it's appropriately `.gitignore`d / submoduled rather than blobbed.
- **5 TODO/FIXME/HACK markers** and **9 `allow(dead_code)`/`allow(unused)`** — small, worth a
  cleanup sweep.

---

---

# Phased Refactoring Plan

## Guiding principles

- **Behavior-preserving only.** Every phase is a structural move. No game logic, wire bytes, or
  formula changes. If a step *requires* a behavior change, stop and escalate — that is a
  separate task.
- **One concern per PR/commit.** Each phase below is sized to land independently and stay
  reviewable. Do not bundle phases.
- **Verify after every step**, not just every phase:
  `rtk cargo check && rtk cargo clippy --all-targets && rtk cargo test`. The full suite
  (currently ~187 core tests) is the safety net for these mechanical moves.
- **Snapshot the public API.** Before Phase 3+, capture `cargo public-api` (or a `grep` of
  `pub use` in `lib.rs`) so re-exports can be kept byte-identical across module moves.
- **Track progress in `tasks/todo.md`** per the always-on workflow rule, and record any
  surprises in `tasks/lessons.md`.

Phases are ordered by **risk-adjusted leverage**: cheap/zero-risk first, god-object last.

---

## Phase 0 — Housekeeping (½ day, zero risk)

**Goal:** remove noise so later diffs are clean.

1. Delete tracked backups: `AGENTS.md.bak`, `config.lua.bak` (keep `config.lua.dist`).
   Add `*.bak` to `.gitignore`.
2. Confirm `reference/` (1.7 GB) and generated `target/`, `log/`, `logs/` are git-ignored;
   if `reference/` is tracked, convert to a submodule or ignore it.
3. Sweep the 5 `TODO/FIXME/HACK` markers and 9 `allow(dead_code)/allow(unused)` — either fix,
   ticket, or document why each must stay.

**Exit criteria:** `git status` clean of stray artifacts; `rtk cargo clippy` warning count
recorded as the new baseline.

---

## Phase 1 — Extract inline tests from mega-files (1–2 days, very low risk)

**Goal:** halve the worst files with pure code-movement, no logic touched.

`idle_stimulus.rs` is ~61% tests, `monster_ai.rs` ~40%. Moving the `#[cfg(test)]` blocks out
is the single highest-leverage low-risk change.

1. For each mega-file, move the trailing `#[cfg(test)] mod tests { … }` into a sibling
   integration/unit test file. Two acceptable patterns:
   - `#[path = "idle_stimulus_tests.rs"] #[cfg(test)] mod tests;` next to the source, or
   - promote to `crates/tfs-rust-core/tests/` if the tests only use the public/`pub(crate)`
     surface (some rely on private items — keep those as `#[path]` siblings).
2. Apply to (in order): `idle_stimulus.rs` (−~3,700 LOC), `monster_ai.rs` (−~1,900),
   `pathfinding.rs` (−~1,000), `sim_harness.rs`, then any other file >50% tests.
3. No renames, no logic edits — diff should be a pure cut/paste plus a `mod` line.

**Exit criteria:** `idle_stimulus.rs` and `monster_ai.rs` each under ~2,500 LOC; identical
test pass count before/after.

---

## Phase 2 — Quarantine simulation/debug code (1 day, low risk)

**Goal:** stop diagnostic scaffolding from shipping in the production `core` API.

1. Introduce a cargo feature `sim` in `tfs-rust-core/Cargo.toml`.
2. Gate `pub mod sim_harness`, `mod chase_debug`, `mod sim_glibc_rand`, and the
   `bin/chase_kite_sim.rs` / `bin/path_compare.rs` targets behind `#[cfg(feature = "sim")]`
   (bins via `required-features`).
3. Ensure the default build (`--no-default-features` and default) excludes them; the sim
   binaries and `tests/` enable `--features sim`.
4. Stretch: if `sim_harness` has no inbound deps from production modules, move it to a new
   `tfs-rust-sim` crate that depends on `core`.

**Exit criteria:** `rtk cargo check -p tfs-rust-core` (default features) compiles without
`sim_harness`/`chase_debug`; sim binaries still build with `--features sim`.

---

## Phase 3 — Rename `_772` core functions (1–2 days, low risk, mechanical)

**Goal:** comply with the always-on `TFS-Core` naming rule (no version suffix on core fns).

Era is already selected by `MechanicsProfile` / `beat_driven_loop`, so the suffix is dead
information. Rename by **behavior**, not version:

| Current | Proposed |
|---------|----------|
| `advance_beat_772` | `advance_beat` |
| `process_creatures_772` | `process_creatures_beat` |
| `process_connections_772` | `process_connections_beat` |
| `tick_ambiente_light_772` | `tick_ambient_light` |
| `monster_on_chase_noway_772` | `monster_on_chase_noway` |
| `monster_move_possible_planning_772` | `monster_move_possible_planning` |
| `monster_exhausted_wait_772` | `monster_exhausted_wait` |
| `clear_todo_772` | `clear_todo` |
| `monster_can_kick_boxes_772` | `monster_can_kick_boxes` |
| `monster_idle_summon_lifecycle_772` | `monster_idle_summon_lifecycle` |

1. Rename one function per commit using a single `replace_all` across the crate; let the
   compiler find all call sites.
2. **Keep** allowed exceptions: config enum variants (`Classic772`), test-only names
   (`*_reads_772`, `test_772_*`), and any literal-wire identifiers.
3. Module file names that are genuinely era-specific transport (`connections_772.rs`,
   `subsystem_counters_772.rs`) may keep the suffix if they encode wire-era transport, but
   their *public functions* should still be behavior-named — decide per file and note in the
   commit.

**Exit criteria:** `grep -rn "fn .*_772" crates/tfs-rust-core/src` returns only test/config
items; clippy clean.

---

## Phase 4 — Split the mega-files into modules (3–5 days, low/medium risk)

**Goal:** turn two monoliths (now test-free after Phase 1) into legible module directories.

### 4a. `idle_stimulus.rs` → `monster_idle/`

Natural seams from the existing function inventory:

```
monster_idle/
  mod.rs          # idle_stimulus dispatch + request_idle_stimulus + state trace
  chase.rs        # monster_idle_chase_*, *_repath, step budget, prepare_combat_chase
  walk_branch.rs  # monster_idle_classify/execute/log_walk_branch, dance/roam/flee arms
  spell_impact.rs # monster_idle_apply_spell_impact, spell_tiles, try_casting, suppress_*
  target.rs       # monster_idle_772_acquire/lose/should_lose_target, roll_strategy
  summon.rs       # monster_idle_summon_lifecycle, summon stubs
  todo_execute.rs # execute_creature_todo_*, run_monster_todo_execute, combat_execute_*
```

### 4b. `monster_ai.rs` → `monster_ai/`

```
monster_ai/
  mod.rs        # re-exports + small helpers (chebyshev, manhattan, is_fleeing, spawn-range)
  on_think.rs   # monster_native_on_think, monster_on_think_target, do_attacking
  follow.rs     # go_to_follow_creature, follow band/repath/reconcile, start_follow_step
  chase.rs      # monster_*_chase_*, greedy/closer step, apply_chase_path
  look.rs       # compute_look_toward_target, monster_update_look_direction
  move_plan.rs  # monster_move_possible_planning, can_walk_to/occupy, tshortway fill
  spawn.rs      # walk_to_spawn, teleport_to_spawn, out_of_spawn_range, leash
```

**Process for each split (keep behavior identical):**
1. Create the directory + `mod.rs` re-exporting the same items; convert `mod monster_ai;` to a
   directory module in `lib.rs`.
2. Move function groups one file at a time, compiling between moves. Keep visibility
   (`pub(crate)`) unchanged so call sites elsewhere don't break.
3. Keep all `impl GameWorld { … }` method *signatures* identical — they can live in split
   files as separate `impl GameWorld` blocks within the same crate.
4. Preserve the `//!` C++ reference headers; copy the relevant references into each new file
   per the `TFS-cpp-references` rule.

**Exit criteria:** no file in either subsystem >~1,200 LOC; `lib.rs` re-exports unchanged;
full test suite green.

---

## Phase 5 — Decompose oversized functions (ongoing, medium risk)

**Goal:** break the 20+ functions over 120 lines into named, testable stages. These read as
state machines, so extract per-stage helpers (per the code-hygiene "extract before
duplication" guidance).

Priority order (highest LOC / hottest path first):

1. `monster_ai.rs::monster_do_attacking` (317) → split target-select / cooldown / cast / move.
2. `walk/mod.rs::on_walk` (287) and `internal_move_creature_step` (206) → per-stage helpers.
3. `monster_ai.rs::go_to_follow_creature` (191).
4. `idle_stimulus.rs::monster_idle_apply_spell_impact` (187).
5. `login_out.rs::enqueue_initial_login_packets_{1098,772}` (207/149) → shared packet-builder
   helpers (note: these legitimately differ by wire era — keep era split, share the common
   scaffolding).

Do these opportunistically alongside Phase 4 when a function lands in a new file anyway.

**Exit criteria:** no production function >~150 LOC without a documented reason.

---

## Phase 6 — Decompose the `GameWorld` god-object (multi-week, highest risk — do last)

**Goal:** reduce the 35-field, 33-file `impl GameWorld` surface so subsystems borrow only what
they need and the `mem::take`/borrow-splitting workarounds disappear.

**Do not attempt before Phases 1–5 land** — they shrink the surface this phase has to move.

Incremental strategy (each sub-step is its own PR):

1. **Group fields into cohesive sub-structs**, leaving `GameWorld` as a thin container:
   | Sub-struct | Fields (from the current 35) |
   |------------|------------------------------|
   | `entities` | `creatures`, `items`, `map`, `container_registry` |
   | `players`  | `player_by_name`, `player_by_guid`, `conn_to_creature` |
   | `net_state`| `pending_outgoing`, `known_creatures_by_conn`, `creature_fully_sent_by_conn`, `deferred_turn_broadcast`, `codec`, `protocol_hooks` |
   | `social`   | `guilds`, `parties`, `party_invites`, `next_party_id` |
   | `world_sys`| `decay`, `spawns`, `houses`, `wildcards`, `stability` |
   | `static_db`| `items_db`, `monsters_db`, `groups`, `vocations`, `mechanics`, `config` |
2. Migrate one subsystem at a time: introduce the sub-struct, update field access via a
   thin accessor, run tests, commit. Use `rustc`'s borrow errors as the to-do list.
3. **Convert `impl GameWorld` methods to free functions** that take the minimal `&mut` slices
   (per the entity-storage rule "pass IDs, not borrowed data"), starting with the leaf
   subsystems (containers, spawns) that have the fewest cross-field reads.
4. Adopt a soft rule going forward: **no new `impl GameWorld` file** — new subsystem logic
   takes explicit borrows.

**Exit criteria:** `impl GameWorld` blocks reduced from 33 files toward the ~10 that truly
need whole-world access; no functional change; full suite green.

---

## Verification & rollback for every phase

- Gate: `rtk cargo check` → `rtk cargo clippy --all-targets` → `rtk cargo test` (and
  `--features sim` after Phase 2). Compare test counts to the pre-phase baseline.
- Because every phase is behavior-preserving, **any** test delta or new clippy warning is a
  signal to stop and review — not to adjust the test.
- Keep each phase on its own branch/commit so a regression can be reverted in isolation.

## Effort summary

| Phase | Effort | Risk | Leverage |
|-------|--------|------|----------|
| 0 Housekeeping | ½ day | none | low |
| 1 Extract tests | 1–2 d | very low | **very high** |
| 2 Quarantine sim | 1 d | low | medium |
| 3 Rename `_772` | 1–2 d | low | medium |
| 4 Split mega-files | 3–5 d | low/med | high |
| 5 Decompose fns | ongoing | medium | medium |
| 6 `GameWorld` | multi-week | **high** | **very high** |
