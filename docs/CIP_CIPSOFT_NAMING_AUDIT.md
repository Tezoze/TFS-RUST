# Cip / CipSoft naming audit

Inventory for a **legal/rebrand rename** of identifiers and user-facing strings that contain `cip`, `Cip`, or `CipSoft`. Generated from repo search on 2026-06-01.

**Scope:** Tracked project sources (`crates/`, `data/`, `docs/`, `.cursor/`, `.agents/`, `tasks/`). Excludes ignored trees (`gameserver/`, `tibia-game-master/`, `src/` TFS C++ mirror) unless noted.

**Re-verify after edits:**

```bash
rg -i 'cipsoft|\bcip\b' --glob '!gameserver/**' --glob '!tibia-game-master/**' --glob '!src/**'
rg 'CipSoft|cipsoft_effective|StepSpeedModel::CipSoft|CipSoftPoly' crates data
```

---

## Summary

| Category | Approx. count | Rename priority |
|----------|---------------|-----------------|
| **Public Rust functions** | 1 | High |
| **Public enum variants** | 2 | High |
| **Lua config string literals** | 1 active + 1 commented | High (shard configs) |
| **Lua parser aliases** (`"cip"`, `"cipsoft"`) | 2 arms | High |
| **Rust test function names** | 7 | Medium |
| **Code references to enum/fn** | ~27 uses across 5 files | High (follow enum/fn rename) |
| **Comment/doc mentions only** | ~120+ lines in 12 `.rs` files; ~84 in 7 `.md`/`.mdc` | Lower (still review for published docs) |

There are **no** modules, crates, or files named `cip*` / `cipsoft*` in this repo.

---

## Tier 1 — Public Rust API (rename first)

These are the only **exported or cross-module symbol names** that embed the trademarked term.

| Current name | Kind | File | Suggested neutral names (pick one set) |
|--------------|------|------|------------------------------------------|
| `cipsoft_effective_speed` | `pub fn` | `crates/tfs-rust-core/src/formulas.rs` | `era772_effective_speed`, `linear_go_effective_speed`, `go_to_get_speed` |
| `StepSpeedModel::CipSoft` | enum variant | `crates/tfs-rust-core/src/formulas.rs` | `StepSpeedModel::LinearGo`, `StepSpeedModel::Era772`, `StepSpeedModel::GoStrength` |
| `LevelExpModel::CipSoftPoly` | enum variant | `crates/tfs-rust-core/src/formulas.rs` | `LevelExpModel::ClassicPoly`, `LevelExpModel::Era772Poly`, `LevelExpModel::DeltaPoly` |

### Call sites for `cipsoft_effective_speed` / `StepSpeedModel::CipSoft`

| File | Role |
|------|------|
| `crates/tfs-rust-core/src/formulas.rs` | Definition, defaults for 772, tests, Lua parse |
| `crates/tfs-rust-core/src/walk.rs` | Walk timers, `wire_step_speed` match arms, tests |
| `crates/tfs-rust-core/src/creature/vocation.rs` | `base_walk_speed`, test |
| `crates/tfs-rust-core/tests/mechanics_formulas.rs` | Shipped `772.lua` parity test |

### Call sites for `LevelExpModel::CipSoftPoly`

| File | Role |
|------|------|
| `crates/tfs-rust-core/src/formulas.rs` | Default 772 profile, Lua parse |
| `crates/tfs-rust-core/src/combat/math.rs` | `exp_for_level` match arm |

---

## Tier 2 — Rust test functions (private)

| Current test name | File |
|-----------------|------|
| `cipsoft_effective_speed_matches_gameserver` | `formulas.rs` |
| `defaults_772_match_cipsoft` | `formulas.rs` |
| `cipsoft_step_duration_matches_notify_go` | `walk.rs` |
| `shipped_772_formulas_match_cipsoft_defaults` | `tests/mechanics_formulas.rs` |
| `fight_mode_modifiers_match_cipsoft_integer_shape` | `combat/math.rs` |
| `probe_value_matches_cipsoft_formula_bounds` | `combat/math.rs` |

Rename together with Tier 1 or drop vendor name from test titles (e.g. `defaults_772_match_profile`, `probe_value_matches_772_bounds`).

---

## Tier 3 — Lua / config strings (user-visible)

| Location | Value | Notes |
|----------|-------|--------|
| `data/formulas/772.lua` | `levelExp = "cipsoft"` | Parsed in `formulas.rs` → `LevelExpModel::CipSoftPoly` |
| `data/formulas/772.lua` | `-- stepSpeedModel = "cipsoft"` | Commented; default is native `CipSoft` without key |
| `crates/tfs-rust-core/src/formulas.rs` | `"cipsoft" \| "cip"` | `stepSpeedModel` parser → `StepSpeedModel::CipSoft` |
| `crates/tfs-rust-core/src/formulas.rs` | `"cipsoft" \| "poly"` | `levelExp` parser → `LevelExpModel::CipSoftPoly` |

**Backward compatibility:** If external shards already use `"cipsoft"` in `772.lua`, keep parser aliases as deprecated aliases after rename, or document a one-time config migration.

---

## Tier 4 — Rust comments only (no symbol rename required)

Files with **comment/docstring** mentions of CipSoft (no extra identifiers beyond Tier 1–2):

| File | ~Matches |
|------|----------|
| `crates/tfs-rust-core/src/formulas.rs` | 36 |
| `crates/tfs-rust-core/src/combat/math.rs` | 17 |
| `crates/tfs-rust-core/src/walk.rs` | 12 |
| `crates/tfs-rust-core/src/monster_ai.rs` | 5 |
| `data/formulas/772.lua` | 5 |
| `crates/tfs-rust-core/src/pathfinding.rs` | 3 |
| `crates/tfs-rust-core/src/creature/vocation.rs` | 3 |
| `crates/tfs-rust-core/tests/mechanics_formulas.rs` | 3 |
| `crates/tfs-rust-core/src/spawn_lifecycle.rs` | 1 |
| `crates/tfs-rust-core/src/login_out.rs` | 1 |
| `crates/tfs-rust-core/src/game_world.rs` | 1 |
| `crates/tfs-rust-core/src/condition.rs` | 1 |
| `crates/tfs-rust-core/src/spell.rs` | 1 |

Typical pattern: `// C++ reference: CipSoft …` or `cract.cc` citations. For legal safety in **distributed** docs, prefer neutral phrasing: **“772 mechanics reference”**, **“decompile outcomes (R12)”**, **“`tibia-game-master` behavior”** — without the vendor name in prose.

---

## Tier 5 — Documentation & Cursor rules

| File | ~Matches | Notes |
|------|----------|--------|
| `docs/PROTOCOL_VERSIONING.md` | 49 | Architecture; many “CipSoft 7.72 source of truth” lines |
| `docs/PROTOCOL_VERSIONING_IMPLEMENTATION_PLAN.md` | 18 | Phase B mechanics |
| `tasks/todo.md` | 5 | |
| `tasks/lessons.md` | 4 | Technical lessons |
| `.agents/rules/tfs-core.md` | 4 | |
| `.cursor/rules/TFS-Core.mdc` | 4 | Mirror of agents rules |
| `.agents/rules/tfs-protocol-versioning.md` | 2 | |
| `.cursor/rules/TFS-protocol-versioning.mdc` | 2 | |
| `.agents/rules/tfs-mechanics-profile.md` | 2 | |
| `.cursor/rules/TFS-mechanics-profile.mdc` | 2 | |

Also see `.gitignore` comment: “CipSoft decompile” for `tibia-game-master/`.

---

## Tier 6 — Related paths (not `cip*` identifiers)

No code symbol, but **directory / legal exposure** when describing the repo:

| Path / term | In repo |
|-------------|---------|
| `tibia-game-master/` | Ignored; cited as 772 mechanics reference |
| `cr*.cc` file citations in comments | Decompile source filenames (retain for internal traceability or redact in public docs) |

---

## Suggested rename strategy

1. **Pick one vocabulary** for the 772 era and use it everywhere in **public** API and config:
   - Examples: `Era772`, `Classic772`, `Mechanics772`, `GoStrength` (for step model), `DeltaPoly` (for level exp).
2. **Rename Tier 1** (`rust-analyzer` / `cargo check` will list all match arms).
3. **Update `772.lua`** and add deprecated parser aliases in `parse_profile` for old `"cipsoft"` / `"cip"` strings if needed.
4. **Rename tests** in the same PR.
5. **Sweep comments/docs** last; use neutral “772 mechanics” in user-facing markdown; keep `cract.cc`-style cites only in internal `//!` blocks if counsel approves.

### Example mapping (illustrative — not applied)

| Old | New (example) |
|-----|----------------|
| `CipSoft` | `Era772` or `LinearGo` |
| `CipSoftPoly` | `Era772Poly` |
| `cipsoft_effective_speed` | `era772_effective_speed` |
| Lua `"cipsoft"` | `"era772"` (+ alias `"cipsoft"` deprecated one release) |

---

## What was excluded from this audit

- **False positives** for substring `cip` inside unrelated words (e.g. `cipher`, `recipient`) — none found in Rust identifiers.
- **Ignored C++ trees** (`gameserver/`, `src/`, `tibia-game-master/`) — may contain vendor strings in upstream comments; not part of Rust crate API.
- **`.cursor/debug-*.log`** — session logs, not source.

---

## Changelog

| Date | Action |
|------|--------|
| 2026-06-01 | Initial audit (post walk-speed fix) |
