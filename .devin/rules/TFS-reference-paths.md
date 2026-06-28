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
| `reference/cipsoft-772/tibia-game-master/src/` | 772 mechanics | AI, chase, combat outcomes |
| `reference/classic-772/tibia-game-master/src/` | 772 mechanics | Same layout as cipsoft-772 (preferred name) |
| `reference/tvp-772/gameserver/src/` | 772 wire | Opcodes, packets, login |
| `src/` (repo root) | 1098 | Default TFS 1.4.2 parity target |

## Agent tooling (mandatory priority for 772 C++)

1. **`cross_repo_search_tool`** — discovery (`ref-772-mechanics`, `ref-772-wire`). Registered by setup script.
2. **`query_graph_tool`** — callers/callees/imports once you have a symbol.
3. **Built-in Read** — explicit `reference/.../src/file.cc` path.
4. **`scripts/ref_grep.sh PATTERN`** — shell text search fallback (`--files-with-matches`, `-C 2`, etc.).

**Never** use built-in Grep/Glob/SemanticSearch to discover 772 C++. **Never** use `rtk grep` on `reference/` (RTK `-l` = max line length, not file list).

## code-review-graph

Main graph indexes **tracked** Rust only. `setup_reference_local.sh` calls `register_reference_graph.sh` when `code-review-graph` is installed — nested local git repos (src/ only) + aliases above.

See `reference/README.md` for layout.
