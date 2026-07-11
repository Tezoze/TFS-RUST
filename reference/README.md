# Local C++ reference trees

Gitignored checkout area for **7.72 parity work**. The Rust server does not require these to build or run at `clientVersion = 772`.

## Layout

```
reference/
├── classic-772/                 # 772 reference stack (preferred name)
│   └── (same layout as legacy cipsoft-772/)
├── retro-772/                 # legacy path — still supported
│   ├── tibia-game-master/       # Mechanics / AI / pathfinding (772 outcomes)
│   ├── tibia-login/             # Login server (port 7171)
│   ├── tibia-querymanager/      # Account / character DB (port 7173)
│   ├── tibia-ipchanger-master/  # Client RSA / server list patcher
│   ├── runtime/                 # Leaked game data (.tibia, dat/, map/, usr/, bin/, …)
│   ├── client/                  # tibia.pem, Tibia772-*.exe
│   └── state/                   # Online stack PID/logs (.tibia-ref-772/ or .tibia-cipsoft/)
├── tvp-772/
│   └── gameserver/              # TVP 7.72 — sole authority for 772 wire/packets
└── archives/                    # tibia-game.tarball.tar.gz, tfs-rust-master.zip, …
```

**1098 reference** stays at repo root: `src/` (TFS 1.4.2 C++).

## Quick start

```bash
# Build + run 772 reference stack (see docs/TIBIA_GAME_MASTER_DEV.md)
scripts/tibia_game_online.sh start

# Override paths if needed
export TFS_REFERENCE_DIR=/other/path/reference
export TIBIA_GAME_DATA=$TFS_REFERENCE_DIR/classic-772/runtime
export TIBIA_RSA_PEM=$TFS_REFERENCE_DIR/classic-772/client/tibia.pem
```

## Role matrix

| Path | Era | Use for |
|------|-----|---------|
| `reference/tvp-772/gameserver/src/` | 772 wire | Opcodes, packets, login, transport |
| `reference/classic-772/tibia-game-master/src/` | 772 mechanics | AI, chase, combat outcomes |
| `src/` | 1098 | Default Rust parity target |

See [docs/PROTOCOL_VERSIONING.md](../docs/PROTOCOL_VERSIONING.md).

## Cursor agents & code-review-graph

Two layers (both required for agent tools):

| Layer | File | Purpose |
|-------|------|---------|
| Git (local) | `.git/info/exclude` | Never commit reference checkouts (`reference/foo/**`) |
| Cursor (tracked) | `.cursorignore` | Re-include **C++ src only** (`tibia-game-master/src/`, `gameserver/src/`) |
| Cursor (tracked) | `.cursorindexingignore` | Skip runtime/archives/log from index upload |

Cursor honors git’s local exclude list. `.cursorignore` un-ignores **only C++ source subtrees** (~200 files), not `runtime/` or `archives/` (~190k game-data files). `.cursorindexingignore` keeps index upload small.

**One-time setup on each clone:**

```bash
scripts/setup_reference_local.sh
```

This configures git exclude, validates `.cursorignore`, and **auto-registers** 772 C++ trees with code-review-graph when `code-review-graph` is installed (`ref-772-mechanics`, `ref-772-wire`). Nested local-only git repos index `src/` only (never pushed).

**Agent discovery (772 C++):** `cross_repo_search_tool` first — not built-in Grep/Glob. Shell text fallback: `scripts/ref_grep.sh PATTERN` (not `rtk grep`; RTK `-l` is max line length).

Re-register manually if needed: `scripts/register_reference_graph.sh`
