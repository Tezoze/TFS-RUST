# TFS-RUST Refactor Audit

**Date:** 2026-06-28
**Re-verified:** 2026-07-01 — all findings re-measured against the current tree; numbers below
refreshed. Every metric held or worsened since the original audit (the debt is self-reinforcing).
**Scope:** Structural health of the Rust workspace — module organization, file/function size,
the `GameWorld` god-object, naming-rule compliance, and test/production separation.
**Out of scope:** Behavioral/parity correctness (see `docs/CODEBASE_AUDIT.md`).

---

## Executive Summary

The port is functionally rich but is accumulating **structural debt** concentrated in
`tfs-rust-core`, which is **54,648 LOC — ~73% of all Rust in the workspace** (74,803 total,
whole-crate including `tests/`/`examples/`). The pain is not spread evenly: a handful of files
and one god-object (`GameWorld`) account for most of the mess.

| Crate | LOC | Health |
|-------|-----|--------|
| `tfs-rust-core` | 54,648 | **Needs work** — god-object + 2 mega-files |
| `tfs-rust-net` | 8,623 | OK — well-split codec/encode |
| `tfs-rust-content` | 5,906 | OK |
| `tfs-rust-common` | 2,013 | OK |
| `tfs-rust-db` | 1,915 | OK |
| `tfs-rust-lua` | 1,698 | OK |

> Only `core` (+2,019) and `net` (+32) grew since the original audit; `content`/`common`/`db`/`lua`
> are byte-for-byte unchanged. Debt is still overwhelmingly a `core` problem.

**Top 5 refactor priorities (highest impact first):**

1. **Split `idle_stimulus.rs` (now 3,060 LOC) and `monster_ai.rs` (now 2,056 LOC).** ✅ Phase 1 done 2026-07-01 — inline tests extracted to `#[path]` sibling files; test pass count & clippy set byte-identical before/after. The remaining split into `monster_idle/` + `monster_ai/` module *directories* (per §1 recommendation) is Phase 4 work — **re-audited 2026-07-11**, see Phase 4 for revised split plans.
2. **Tame the `GameWorld` god-object** — `impl GameWorld` is spread across **34 files**.
3. **Move simulation/debug harness code out of the production `core` library.** ✅ Phase 2 done 2026-07-02 — `sim_harness`, `chase_debug`, and `sim_glibc_rand` sim parts gated behind `#[cfg(any(test, feature = "sim"))]` with no-op stubs for production. `chase_kite_sim` bin requires `--features sim`.
4. **Fix `_772` naming-rule violations on core functions** (259 identifiers, 110 on `fn`s). ✅ Phase 3 done 2026-07-10 — all production `_772`-suffixed functions renamed to behavior-based names across core, content, lua, and net crates. Remaining `_772` identifiers are test fns, config, data constants, and local variables (all allowed exceptions).
5. **Decompose oversized functions** (20+ functions over 120 lines, longest is 317).

---

## 1. Mega-files: two files dominate

**Re-measured 2026-07-11.** Phase 1 extracted inline tests to `#[path]` sibling files; the
unified beat engine work deleted 1098 AI. Both mega-files are now test-free but still
monolithic.

| File | Current LOC | Orig LOC | `fn` count | Notes |
|------|-------------|----------|-----------|-------|
| `idle_stimulus.rs` | 3,060 | 6,809 | 52 | All `impl GameWorld`; tests in `idle_stimulus_tests.rs` (5,937) |
| `monster_ai.rs` | 2,056 | 4,758 | 38 | 8 free + 30 `impl GameWorld`; tests in `monster_ai_tests.rs` + `monster_ai_world_tests.rs` |
| `walk/mod.rs` | 2,655 | 2,285 | 46 | Grew since audit |
| `creature_todo.rs` | 2,048 | — | — | **New** — not in original audit; 3rd largest in core |
| `pathfinding.rs` | 1,231 | 2,261 | — | Shrank (tests extracted) |
| `game_world_inventory.rs` | 1,288 | 964 | — | Grew +34% |
| `player/inventory/query_add.rs` | 1,280 | 1,184 | — | In `player/` module (already refactored) |
| `player/combat/ranged.rs` | 1,253 | — | — | **New** — in `player/` module |
| `game_world_chat.rs` | 1,163 | — | — | **New** — created during CH phases |
| `game_world_spectators.rs` | 1,089 | — | — | **New** — split from game_world.rs |

### Findings

- `idle_stimulus.rs` is still a **single ~133 KB file** holding 52 functions. It is effectively
  a sub-system, not a module. The IDE chokes on it and reviewers cannot reason about it.
- Both mega-files mix several concerns: idle/chase stepping, spell impact application,
  walk-branch execution, target selection, combat damage, and todo execution all live together.
- **The tail has grown.** The original audit named `monster_push.rs` (then 1,545) as the only
  other file over 1,200 LOC. It has since shrunk to 565. But **4 new files** now cross 1,200 LOC
  (`creature_todo.rs`, `game_world_inventory.rs`, `player/inventory/query_add.rs`,
  `player/combat/ranged.rs`), and **2 more** cross 1,000 (`game_world_chat.rs`,
  `game_world_spectators.rs`). The player files are already in the `player/` module directory
  (refactored separately) and are single-concern, so they're acceptable. The remaining
  second-tier candidates for Phase 4/5 are `creature_todo.rs`, `game_world_inventory.rs`, and
  `game_world_chat.rs` — see Phase 4c.

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

`GameWorld` has **~51 public fields** (up from 35 at the original audit) and its `impl` blocks
are scattered across **34 source files**. Method counts per file (inside `impl GameWorld`):

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

There are **259** `_772`-suffixed identifiers in `tfs-rust-core/src` (110 of them on `fn`
definitions). Many are on core game functions, which directly violates the project's own
always-on rule
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

- **`unsafe` surface** — 21 `unsafe` uses in core, confined to three files:
  `lua_scope.rs` (15 — the sanctioned re-entrant `&mut GameWorld` scope required by
  `tfs-lua-boundaries.md`), `sim_glibc_rand.rs` (5 — parity RNG, diagnostic-only) and
  `game_world.rs` (1). This is *correctly localized* per the rules, but the audit should track
  it: Phase 2 (sim quarantine) removes the 5 `sim_glibc_rand` cases from the production build,
  and Phase 6 (god-object) is the only thing that can shrink the 15 in `lua_scope.rs` — those
  exist *because* subsystems re-borrow the whole `GameWorld`. Treat "no new `unsafe` outside
  `lua_scope.rs`" as a standing invariant.
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

## 8. What this audit does *not* measure (scope honesty)

The findings above are size/shape/naming metrics that are cheap to quantify mechanically. Two
axes are deliberately **not** covered and should not be assumed clean:

- **Code duplication is unquantified.** The `TFS-code-hygiene` rule is largely about
  "extract before duplication," yet this audit never ran clone detection. The mega-files and the
  `game_world_*` / `monster_*` sibling clusters are exactly where copy-paste tends to hide. Before
  Phase 4, run a clone pass (e.g. a token-based duplication check) so splits *consolidate*
  duplicates instead of scattering them across new module files.
- **Behavioral / parity correctness is out of scope** — that lives in `docs/CODEBASE_AUDIT.md`.
  This document assumes the current behavior is the reference; every phase preserves it.

These are gaps in *this* audit, not clean bills of health.

---

# High-Level Roadmap (condensed)

The whole effort rests on one invariant: **every phase is behavior-preserving** — no wire
bytes, formulas, or game logic change. Gate each step with
`rtk cargo check && rtk cargo clippy --all-targets && rtk cargo test`; any test-count or clippy
delta means *stop*, not *adjust the test*. Stages are ordered so leverage-per-risk descends and
each stage shrinks the surface the next has to move.

| Stage | Phases | Effort | Risk | Outcome |
|-------|--------|--------|------|---------|
| **A — Clear the noise** | 0, 1 | ~2–3 d | ~zero | Backups/markers gone; inline tests pulled out — roughly halves the two mega-files by pure cut/paste. **Phase 1 ✅ done 2026-07-01** (Phase 0 still pending). |
| **B — Shrink the surface** | 2, 3 | ~2–4 d | low | Sim/debug code behind a `sim` feature; `_772` core fns renamed by behavior (compiler-guided). **Phase 2 ✅ done 2026-07-02; Phase 3 ✅ done 2026-07-10** |
| **C — Make monoliths legible** | 4, 5 | ~1–2 wk | low/med | `idle_stimulus.rs`/`monster_ai.rs` become module dirs; 20+ oversized fns split into per-stage helpers |
| **D — Tame the god-object** | 6 | multi-wk | **high** | ~51 `GameWorld` fields grouped into sub-structs; `impl` methods become free fns on minimal borrows — kills `mem::take`/borrow-splitting |

**Why this order:** Stage A is the highest-leverage, lowest-risk work (test extraction alone
halves the worst files). Stage D is the real prize but is only tractable once A–C remove the
mega-file bulk and `_772` churn. Do **not** reorder — D before A/B multiplies the god-object's
blast radius. The detailed, per-phase execution plan follows.

---

## Guiding principles

- **Behavior-preserving only.** Every phase is a structural move. No game logic, wire bytes, or
  formula changes. If a step *requires* a behavior change, stop and escalate — that is a
  separate task.
- **One concern per PR/commit.** Each phase below is sized to land independently and stay
  reviewable. Do not bundle phases.
- **Verify after every step**, not just every phase:
  `rtk cargo check && rtk cargo clippy --all-targets && rtk cargo test`. The core suite is the
  safety net — see **Baseline metrics** below for the exact frozen counts.
- **Snapshot the public API.** Before Phase 3+, capture `cargo public-api` (or a `grep` of
  `pub use` in `lib.rs`) so re-exports can be kept byte-identical across module moves.
- **Track progress in `tasks/todo.md`** per the always-on workflow rule, and record any
  surprises in `tasks/lessons.md`.

Phases are ordered by **risk-adjusted leverage**: cheap/zero-risk first, god-object last.

---

## Baseline metrics (frozen 2026-07-01)

These are the "green" reference points every behavior-preserving phase must reproduce. Because
the tree drifts, **re-capture and re-freeze these at the moment Phase 0 actually starts** — then
treat any deviation during Phases 1–6 as a stop-and-review signal, not a test to adjust.

| Metric | Baseline | Command |
|--------|----------|---------|
| Core tests | **482 passed, 2 ignored** (12 suites, ~85 s) | `rtk cargo test -p tfs-rust-core` |
| Clippy (whole workspace, clean build) | **46 `^warning:` lines / 44 unique warnings** (re-measured 2026-07-01, all pre-existing) | `cargo clippy --all-targets 2>&1 \| grep '^warning:' \| sort` |
| Ignored tests | 2 (document why each is ignored before Phase 1) | — |

Notes:
- The original audit's "~187 core tests" was **stale**; the real current count is 482 passing.
- **Clippy baseline corrected (Phase 1, 2026-07-01):** the original audit's "0 warnings — No
  issues found" claim is **stale**. A clean-build re-measurement via
  `cargo clippy --all-targets 2>&1 | grep '^warning:' | sort` yields **46 `^warning:` lines /
  44 unique warnings**, all pre-existing (`too_many_arguments` in `chase_debug.rs` /
  `monster_ai.rs`; `dead_code` on test-only-called fns like `monster_has_melee_attack_spell`,
  `monster_idle_roll_strategy`, `execute_creature_todo_go`, `parity_random_shuffle`,
  `SimGlibcRng`; `drop`-of-reference in `process_skills.rs`; etc.).
- **Do NOT diff `rtk cargo clippy` aggregated summaries for regression checks** — its output is
  non-deterministic across runs (it prints a rotating subset of locations per rule; untouched
  files appear/disappear). Use raw `cargo clippy ... | grep '^warning:' | sort` instead.
  (See `tasks/lessons.md` #96.)
- The bar for Phases 2–6 is "no NEW warning vs. the live captured baseline above," not
  "0 warnings." Phase 1 verified its extraction produced a **byte-identical** warning set
  before/after.

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

## Phase 1 — Extract inline tests from mega-files (1–2 days, very low risk) ✅ DONE 2026-07-01

> **Status: COMPLETE.** All exit criteria met. See "Phase 1 results" below.

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

### Phase 1 results

Pattern used: `#[cfg(test)] #[path = "<file>_tests.rs"] mod <name>;` — the `#[path]` file
holds the module **body** (bytes between `mod <name> {` and its closing `}`). The extracted
module stays a child of the source module, so `use super::*` / private-item access and
`crate::<src>::<tests>::*` filter paths are preserved exactly. All extracted tests rely on
private items, so `tests/` integration promotion was not viable — `#[path]` siblings in
`src/` is the correct pattern.

| Source file | Before | After | Test file(s) created |
|-------------|--------|-------|----------------------|
| `idle_stimulus.rs` | 6,809 | 2,511 | `idle_stimulus_tests.rs` (4,299) |
| `monster_ai.rs` | 4,758 | 2,843 | `monster_ai_tests.rs` (200) + `monster_ai_world_tests.rs` (1,716) |
| `pathfinding.rs` | 2,261 | 1,230 | `pathfinding_tests.rs` (1,031) |
| `sim_harness.rs` | 2,441 | 1,788 | `sim_harness_tests.rs` (653) |
| `todo_queue.rs` | 293 | 124 | `todo_queue_tests.rs` (169) |
| `monster_push.rs` | 1,545 | 639 | `monster_push_tests.rs` (906) |
| `spell.rs` | 299 | 137 | `spell_tests.rs` (162) |
| `creature_think.rs` | 539 | 256 | `creature_think_tests.rs` (283) |

`monster_ai.rs` had **two** inline test mods (`tests` + `world_tests`); each was extracted to
its own `#[path]` file with its own `mod <name>;` declaration to preserve the nested module
path (`crate::monster_ai::{tests,world_tests}::*`) and existing test filters — do not collapse
them into one wrapper module.

**Verification:**
- `rtk cargo test -p tfs-rust-core` → **482 passed, 2 ignored, 12 suites** — byte-identical to
  the pre-Phase-1 baseline. ✅
- `cargo clippy --all-targets 2>&1 | grep '^warning:' | sort` diffed before/after via
  `git stash` → **byte-identical** (46 warning lines / 44 unique in both). **Zero clippy
  regression.** ✅
- `rtk cargo check -p tfs-rust-core` after each file → 0 errors throughout. ✅
- Exit criteria: `idle_stimulus.rs` ≤~2,500 (2,511), `monster_ai.rs` at audit-measured prod
  LOC ~2,835 (2,843). ✅

**Lessons captured** (`tasks/lessons.md` #95, #96):
- `#[cfg(test)] #[path] mod <name>;` extraction semantics (the `#[path]` file holds the body,
  not the wrapper).
- **`rtk cargo clippy` aggregated output is non-deterministic** across runs (rotating subset of
  locations per rule; untouched files appear/disappear). Do NOT diff rtk summaries for
  regression checks — use raw `cargo clippy ... | grep '^warning:' | sort`.
- The audit's "Clippy is currently clean — 0 warnings" baseline claim (line ~293) is **stale**:
  the live baseline is **46 `^warning:` lines / 44 unique warnings**, all pre-existing. The bar
  for Phases 2–6 is "no NEW warning vs. the live captured baseline," not "0 warnings."

---

## Phase 2 — Quarantine simulation/debug code (1 day, low risk) ✅ DONE 2026-07-02

> **Status: COMPLETE.** All exit criteria met. See "Phase 2 results" below.

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

### Phase 2 results

**Key discovery:** the audit's framing of `chase_debug` and `sim_glibc_rand` as pure "diagnostic
scaffolding" was partially incorrect. Both are called from 6+ production files. `sim_glibc_rand`
contains production code: `GlibcRngState` (the `GameWorld::parity_rng` field for the 772 beat-driven
loop), `DANCE_DIR_ORDER` (monster AI constant), and `parity_random/rand_mod/random_shuffle`
(production RNG dispatchers that use `thread_rng` when sim mode is off). A simple `#[cfg(feature =
"sim")]` gate on the whole module would break production compilation.

**Approach:** `#[cfg(any(test, feature = "sim"))]` gates the full implementation;
`#[cfg(not(any(test, feature = "sim")))]` provides no-op stubs with identical signatures. `cargo
test` auto-enables via `cfg(test)` — no workflow change. Production builds compile stubs only.

| Module | Always compiled | `#[cfg(any(test, feature = "sim"))]` |
|--------|----------------|--------------------------------------|
| `sim_harness` | — | Entire module (test/diagnostic only) |
| `chase_debug` | Stubs (all `log_*` → no-op, `chase_path_debug_enabled() → false`) | Full JSONL trace implementation |
| `sim_glibc_rand` | `GlibcRngState`, `DANCE_DIR_ORDER`, `parity_*` dispatchers, `sim_glibc_rng_enabled() → false`, `sim_rng_trace_site` (no-op guard) | `sim_random`, `sim_rand_mod`, `SimGlibcRng`, `enable_sim_glibc_rng`, `resync_harness_glibc_rng_from_env`, trace functions, `sim_probe_random_factor`, `harness_melee_realign_*`, `draw_rand` |

**Production call site changes** — `#[cfg(any(test, feature = "sim"))]` on sim-only branches
inside `if sim_glibc_rng_enabled()` blocks:
- `combat/math.rs` (2 sites: probe random factor + armor roll)
- `creature/monster_combat.rs` (1 site: poison damage sim path)
- `monster_ai.rs` (1 site: melee realign block)
- `game_world.rs` (3 sites: `init_sim_rng_from_env` sim parts, `resync_sim_glibc_rng`, `sim_dance_choice` sim branch)

**Exposed dead code** — gating `sim_harness` revealed functions only kept alive by the
`chase_kite_sim` bin: `search_login_field` / `spiral_login_positions` / `harness_place_creature_login`
in `spawn_placement.rs` (gated with `#[cfg(any(test, feature = "sim"))]`), `player_apply_spell_exhaust`
and `parity_random_shuffle` method in `game_world.rs` (`#[allow(dead_code)]` — pre-existing dead code).

**Verification:**
- `cargo check -p tfs-rust-core` (default) → 0 errors, no sim code compiled. ✅
- `cargo test -p tfs-rust-core --lib` → **481 passed, 2 ignored** — identical to pre-Phase-2 baseline. ✅
- `cargo check -p tfs-rust-core --features sim` → 0 errors. ✅
- `cargo check --bin chase_kite_sim --features sim` → 0 errors. ✅
- `cargo check --bin path_compare` (no sim) → 0 errors. ✅
- `cargo check --bin chase_kite_sim` (no sim) → correctly fails (`requires the features: sim`). ✅
- Clippy net warnings **decreased**: lib 44→29, test 76→75. No new unique warnings. ✅

**Lessons captured** (`tasks/lessons.md` #100):
- `chase_debug` and `sim_glibc_rand` are NOT pure diagnostic — they have production callers.
- `#[cfg(any(test, feature = "sim"))]` + stub pattern lets `cargo test` work without `--features sim`.
- `parity_random` uses `#[cfg]` on the sim branch, falling through to `thread_rng` in production.
- Gating sim bins exposes pre-existing dead code that was only alive via the bin's dependency.

---

## Phase 3 — Rename `_772` core functions (1–2 days, low risk, mechanical) ✅ DONE 2026-07-10

> **Status: COMPLETE.** All exit criteria met. See "Phase 3 results" below.

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

### Phase 3 results

**Key discovery:** the audit's 10 example functions (`advance_beat_772`, `process_creatures_772`,
`process_connections_772`, `tick_ambiente_light_772`, `monster_on_chase_noway_772`,
`monster_move_possible_planning_772`, `monster_exhausted_wait_772`, `clear_todo_772`,
`monster_can_kick_boxes_772`, `monster_idle_summon_lifecycle_772`) had **already been renamed**
in prior work — they appear only in a stale comment at `walk/mod.rs:601`. The remaining
production `_772` functions were spread across **four crates**, not just `tfs-rust-core`:

| Crate | Renamed | Call sites updated |
|-------|---------|--------------------|
| core | `condition_type_from_lua_772` → `condition_type_from_lua` | `game_world_chat.rs` (2), `game_world_script.rs` (1) |
| content | `is_terrain_bank_772` → `is_terrain_bank` | `monster_ai.rs` (2), `monster_push.rs` (1) |
| content | `is_unpass_772` → `is_unpassable` | `monster_ai.rs` (2), `monster_push.rs` (2) |
| content | `is_unmove_772` → `is_immovable` | `monster_ai.rs` (2), `monster_push.rs` (2) |
| content | `is_avoid_hazard_772` → `is_avoid_hazard` | `monster_ai.rs` (1), `monster_push.rs` (1) |
| content | `avoid_damage_type_772` → `avoid_damage_type` | `monster_ai.rs` (1), `monster_push.rs` (1) |
| content | `waypoints_raw_772` → `waypoints_raw` | `items.rs` (1) |
| content | `reference_772_objects_srv_under` → `reference_objects_srv_under` | `objects_srv.rs` (4) |
| lua | `return_value_message_772` → `return_value_message` | `userdata/player.rs` (8) |
| net | `send_icons_772` → `send_icons_classic` | `outgoing_extra.rs`, `login_out.rs` (2) |
| net | `liquid_color_772` → `liquid_color` | `codec/v772.rs` (1) |
| net | `build_login_success_772` → `build_login_success_classic` | `protocol_login_out.rs` (1) |
| net | `premium_days_left_772` → `premium_days_left` | `protocol_login_out.rs` (4) |

**Naming choices:**
- `is_unpass` → `is_unpassable`, `is_unmove` → `is_immovable` — more idiomatic Rust adjectives.
- `send_icons_772` → `send_icons_classic` — the 772 variant writes a `u8` icon field vs the
  1098 `u16`; "classic" describes the narrow-field behavior, not the version number.
- `build_login_success_772` → `build_login_success_classic` — paired with `_1098` (future
  rename to `_modern` in a follow-up).

**Exceptions kept (allowed by the rule):**
- Test fns: `test_772_*`, `test_phase9_772_*`, `*_reads_772`, `place_bag_on_tile_772`,
  `otclient_772_*`, `real_772_client_*`, `wire_step_speed_772_*`, etc.
- Config: `protocol_version_reads_772` (`config.rs`).
- Data constants: `OTB_MAJOR_772` (literal OTB version 2), `LOGIN_ERR_772` (literal wire
  byte `0x0A`), `REF_772_DIR_NAMES` (literal directory path strings).
- Local variables: `is_772`, `codec_772` (not public APIs).

**Out of scope (follow-up):** `_1098`-suffixed production functions (`send_player_stats_1098`,
`send_player_skills_1098`, `send_basic_data_1098`, `build_login_success_1098`,
`enqueue_initial_login_packets_1098`) are also naming-rule violations but were not in Phase 3's
scope as written.

**Verification:**
- `cargo test -p tfs-rust-core --lib` → **585 passed, 2 ignored** — identical to pre-Phase-3
  baseline. ✅
- `cargo clippy --all-targets 2>&1 | grep '^warning:' | sort | wc -l` → **26** — identical to
  baseline. ✅
- `grep -rn "fn .*_772" crates/tfs-rust-core/src` → 86 matches, **all test/config items**. ✅
- `grep -rn "fn .*_772" crates/tfs-rust-{content,lua,net}/src` → all test items. ✅
- `cargo check --all-targets` → 0 errors. ✅

---

## Phase 4 — Split the mega-files into modules (3–5 days, low/medium risk)

**Re-audited 2026-07-11** against the current tree. Both files are now test-free (Phase 1
extracted tests to `#[path]` sibling files). The unified beat engine work (Phases 3–10 in git
log) deleted the 1098 reactive AI and `beat_driven_loop` flag, which removed several functions
the original Phase 4 plan referenced. The split plans below reflect the **actual** current
function inventory.

**Current LOC:**
| File | Audit LOC (orig) | Current LOC | `fn` count |
|------|------------------|-------------|-----------|
| `idle_stimulus.rs` | 6,809 | 3,060 | 52 (all `impl GameWorld`) |
| `monster_ai.rs` | 4,758 | 2,056 | 38 (8 free + 30 `impl GameWorld`) |

### 4a. `idle_stimulus.rs` → `monster_idle/`

The file holds 52 functions across 11 logical groups. The original plan placed only ~7 files
and missed 3 entire groups (combat/damage, attack enqueue, utilities). Revised split:

```
monster_idle/
  mod.rs          # dispatch: idle_stimulus, player_idle_stimulus, request_idle_stimulus
                  #   + enums (MonsterIdleWalkBranch, MonsterIdleWalkOutcome, TodoExecuteKind)
  combat.rs       # combat_execute_with_stimulus, apply_mana_shield, player_absorb_percent,
                  #   clear_nonplayer_invisibility, monster_damage_stimulus  (~338 LOC)
  summon.rs       # monster_idle_summon_lifecycle  (~115 LOC)
  target.rs       # monster_idle_acquire_target, monster_idle_lose_existing_target,
                  #   monster_idle_should_lose_target, monster_cast_target_id,
                  #   monster_idle_roll_strategy_from_roll  (~203 LOC)
  spell.rs        # monster_idle_apply_spell_impact, monster_idle_try_casting,
                  #   monster_idle_spell_tiles, monster_idle_suppress_adjacent_melee_spell
                  #   (~346 LOC)
  core.rs         # monster_idle_stimulus, monster_idle_stimulus_after_creature_move,
                  #   monster_idle_stimulus_inner, monster_idle_reschedule_target_bound_if_parked,
                  #   monster_idle_reset_combat_state, monster_idle_try_talk  (~238 LOC)
  chase.rs        # monster_idle_maybe_enter_attacking, prepare/set_combat_chase_mode,
                  #   emit_combat_state, chase_needs_repath, classify/execute/log_walk_branch,
                  #   master_follow_hold_or_wait, noway_clear_and_roam,
                  #   prepare_and_enqueue_go  (~458 LOC)
  attack.rs       # monster_enqueue_todo_attack_actions, monster_idle_can_enqueue_attack,
                  #   monster_idle_rotate_toward_attack_target, monster_execute_rotate_toward,
                  #   monster_idle_maybe_enqueue_attack, monster_combat_handle_close_chase_blocked
                  #   (~258 LOC)
  todo_execute.rs # execute_creature_todo_action, execute_player_use, execute_player_move,
                  #   execute_creature_todo_go, finish_creature_todo_execute,
                  #   run_monster_todo_execute, maybe_idle_stimulus_after_go_complete
                  #   (~611 LOC)
  utils.rs        # monster_state_trace_str, monster_sleep_wake_on_creature_move,
                  #   monster_idle_maybe_enqueue_at_goal_wait, monster_exhausted_wait
                  #   (~72 LOC)
```

**Cross-group coupling notes** (affects move order):
- `core.rs` (Group F) is the central hub — calls into target, spell, chase, attack, utils.
  Move it last, after its callees are in place.
- `todo_execute.rs` (Group J) calls back into `core.rs` and `spell.rs`. This bidirectional
  dependency is fine within the same crate (separate `impl GameWorld` blocks).
- `combat.rs` (Group B) is relatively isolated — safe to move first.

### 4b. `monster_ai.rs` → `monster_ai/`

**Stale references removed:** the original plan referenced `monster_native_on_think`,
`monster_on_think_target`, `go_to_follow_creature`, `start_follow_step`, `teleport_to_spawn`,
and `out_of_spawn_range` — all deleted during the unified beat engine work. The `on_think.rs`
and `follow.rs` files would be empty. Revised split against the actual 38-function inventory:

```
monster_ai/
  mod.rs        # re-exports + free helpers: chebyshev, manhattan,
                #   monster_idle_chase_step_budget, monster_master_follow_in_wait_band  (~22 LOC)
  attack.rs     # monster_do_attacking (376 LOC — Phase 5 candidate), monster_tile_in_protection_zone
                #   (~381 LOC)
  chase.rs      # 18 chase/follow core fns: monster_idle_chase_repath, monster_on_chase_noway,
                #   monster_idle_dance_step, monster_idle_master_follow,
                #   monster_combat_enqueue_close_chase_go, monster_chase_stalled_without_wakeup,
                #   monster_combat_reschedule_if_stalled, etc.  (~536 LOC)
  move_plan.rs  # monster_try_apply_chase_path, monster_path_search_params,
                #   get_creature_path_to_with_fpp, monster_move_possible_planning (156 LOC),
                #   monster_tshortway_fill_walkable, monster_can_occupy_chase_tile,
                #   monster_roam_leash_radius, monster_can_walk_to  (~429 LOC)
  movement.rs   # monster_idle_roam_step, monster_idle_flee_step,
                #   monster_should_keep_chase/dance_walk_alive, monster_walk_to_spawn,
                #   monster_on_walk_complete, monster_next_walk_step  (~248 LOC)
  look.rs       # compute_look_toward_target, monster_update_look_direction  (~36 LOC)
  spawn.rs      # is_fleeing, is_in_spawn_range, is_within_walk_to_spawn_range  (~60 LOC)
  debug.rs      # fillmap_terrain_waypoints_at, fillmap_waypoints_at,
                #   dump_tshortway_fill_walkable_viewport — #[cfg(test)] or sim feature  (~67 LOC)
```

**Cross-group coupling notes:**
- `chase.rs` ↔ `move_plan.rs` have significant cross-calls (repath → path search → apply).
- `chase.rs` → `movement.rs` interconnected through the chase lifecycle.
- `attack.rs` is relatively independent (calls `look.rs` only).
- External deps: `monster_events.rs` (`monster_on_follow_creature_complete`, called 4× from
  this file), `monster_targets.rs` (`monster_sight_clear`, `monster_throw_possible`),
  `idle_stimulus.rs` (`monster_execute_rotate_toward`).

### 4c. New mega-files not in the original audit

Three files have appeared or grown into the mega-file range since the audit was written:

| File | Current LOC | Status | Recommendation |
|------|-------------|--------|----------------|
| `creature_todo.rs` | 2,048 | Not in audit — now 3rd largest in core | Single-concern (ToDo queue data structure). Likely stays as one file; if it must shrink, split by action variant (ToDoGo / ToDoAttack / ToDoSpell / ToDoTurn / ToDoWait). |
| `game_world_inventory.rs` | 1,288 | Was 964 (+34%) | Multi-concern: inventory slot ops + Lua item hooks + depot + look-at. Candidate for split into `game_world_inventory_ops.rs` + `game_world_inventory_lua.rs`. |
| `game_world_chat.rs` | 1,163 | Not in audit — created during CH phases | Multi-concern: SAY/WHISPER/YELL + PRIVATE + CHANNEL + BROADCAST. Candidate for split by talk-type groups. |

**Scope decision needed:** include these in Phase 4, or defer to Phase 5? The two mega-file
splits (4a/4b) are the priority; 4c can follow opportunistically.

**Process for each split (keep behavior identical):**
1. Create the directory + `mod.rs` re-exporting the same items; convert `mod idle_stimulus;`
   / `mod monster_ai;` to directory modules in `lib.rs`.
2. Move function groups one file at a time, compiling between moves. Keep visibility
   (`pub(crate)`) unchanged so call sites elsewhere don't break.
3. Keep all `impl GameWorld { … }` method *signatures* identical — they can live in split
   files as separate `impl GameWorld` blocks within the same crate.
4. Preserve the `//!` C++ reference headers; copy the relevant references into each new file
   per the `TFS-cpp-references` rule.
5. Move leaf/isolated groups first (combat, summon, utils, look, spawn); move the hub
   (`core.rs` / `chase.rs`) last after its callees are in place.

**Exit criteria:** no file in either subsystem >~1,200 LOC; `lib.rs` re-exports unchanged;
full test suite green. (Note: `creature_todo.rs` at 2,048 LOC is single-concern and may
remain — document the exception if it stays.)

---

## Phase 5 — Decompose oversized functions (ongoing, medium risk)

**Goal:** break the 20+ functions over 120 lines into named, testable stages. These read as
state machines, so extract per-stage helpers (per the code-hygiene "extract before
duplication" guidance).

Priority order (highest LOC / hottest path first):

1. `monster_ai.rs::monster_do_attacking` (376) → split target-select / cooldown / cast / move.
2. `walk/mod.rs::on_walk` (287) and `internal_move_creature_step` (206) → per-stage helpers.
3. `idle_stimulus.rs::monster_idle_apply_spell_impact` (171).
4. `idle_stimulus.rs::execute_creature_todo_action` (407) → split per ToDo action variant.
5. `idle_stimulus.rs::monster_idle_stimulus_inner` (118) → extract target/spell/chase stages.
6. `login_out.rs::enqueue_initial_login_packets_{1098,772}` (207/149) → shared packet-builder
   helpers (note: these legitimately differ by wire era — keep era split, share the common
   scaffolding).

> **Removed from original list:** `monster_ai.rs::go_to_follow_creature` (191) — deleted during
> the unified beat engine work. `monster_do_attacking` grew from 317 to 376 LOC.

Do these opportunistically alongside Phase 4 when a function lands in a new file anyway.

**Exit criteria:** no production function >~150 LOC without a documented reason.

---

## Phase 6 — Decompose the `GameWorld` god-object (multi-week, highest risk — do last)

**Goal:** reduce the 35-field, 33-file `impl GameWorld` surface so subsystems borrow only what
they need and the `mem::take`/borrow-splitting workarounds disappear.

**Do not attempt before Phases 1–5 land** — they shrink the surface this phase has to move.

Incremental strategy (each sub-step is its own PR):

1. **Group fields into cohesive sub-structs**, leaving `GameWorld` as a thin container. The
   field set has grown to ~51 since the original audit, so the grouping below now also absorbs
   the newer `pub(crate)` beat-loop/RNG/scheduler fields:
   | Sub-struct | Fields (from the current ~51) |
   |------------|------------------------------|
   | `entities` | `creatures`, `items`, `map`, `container_registry` |
   | `players`  | `player_by_name`, `player_by_guid`, `conn_to_creature` |
   | `net_state`| `pending_outgoing`, `known_creatures_by_conn`, `creature_fully_sent_by_conn`, `deferred_turn_broadcast`, `codec`, `protocol_hooks`, `next_statement_id`, `pending_idle_kick_772` |
   | `social`   | `guilds`, `parties`, `party_invites`, `next_party_id` |
   | `world_sys`| `decay`, `spawns`, `houses`, `wildcards`, `stability`, `spawn_slot_by_creature`, `last_creature_bucket_tick`, `creatures_pending_release`, `items_pending_release` |
   | `static_db`| `items_db`, `monsters_db`, `groups`, `vocations`, `mechanics`, `config`, `db`, `monster_world_config`, `connection_config` |
   | `beat_loop`| `tick_counter`, `server_ms`, `beat_driven_loop`, `todo_queue`, `walk_wake_tx`, `subsystem_counters_772`, `round_nr_772`, `last_ambiente_brightness`, `lag_772`, `monster_viewport_notify_depth` |
   | `rng`      | `ai_rng`, `parity_rng` |
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
| 4 Split mega-files | 4–6 d | low/med | high |
| 5 Decompose fns | ongoing | medium | medium |
| 6 `GameWorld` | multi-week | **high** | **very high** |
