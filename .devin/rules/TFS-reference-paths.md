---
trigger: model_decision
description: Local 772 C++ reference trees — agent access and graph lookup
globs: crates/tfs-rust-core/**/*.rs, docs/**/*772*, reference/**/*
---

# Local C++ reference trees (`reference/`)

Excluded locally via `.git/info/exclude` — **never committed**. Tracked `.cursorignore` hierarchical negations re-include **C++ source only** (not `runtime/`, `archives/`, or `client/`).

Run `scripts/setup_reference_local.sh` once per clone (also auto-registers code-review-graph).

## Paths (agent-accessible after setup)

| Path | Era | Use for |
|------|-----|---------|
| `reference/cipsoft-772/tibia-game-master/src/` | 772 mechanics | AI, chase, combat outcomes, config constants |
| `reference/tvp-772/gameserver/src/` | 772 wire (primary) | Opcodes, packets, login |
| `src/` (repo root) | 1098 / 772 cross-ref | TFS 1.4.2 parity target; opcode cross-reference for 772 |

## Agent tooling (mandatory priority for 772 C++)

1. **Built-in Grep/Read** — primary discovery and file access for all reference trees.
2. **`scripts/ref_grep.sh PATTERN`** — shell text search fallback for 772 reference trees (`--files-with-matches`, `-C 2`, etc.).

**Never** use `rtk grep` on `reference/` (RTK `-l` = max line length, not file list).

## code-review-graph

Main graph indexes **tracked** Rust only. `setup_reference_local.sh` calls `register_reference_graph.sh` when `code-review-graph` is installed — nested local git repos (src/ only) + aliases above.

See `reference/README.md` for layout.
