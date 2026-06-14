# Local C++ reference trees

Gitignored checkout area for **7.72 parity work**. The Rust server does not require these to build or run at `clientVersion = 1098`.

## Layout

```
reference/
├── classic-772/                 # 772 reference stack (preferred name)
│   └── (same layout as legacy cipsoft-772/)
├── cipsoft-772/                 # legacy path — still supported
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

Reference checkouts are **excluded locally** (`.git/info/exclude`, not shared `.gitignore`) so they are never committed but remain visible to Cursor agents. After cloning reference trees here, agents can `Read` / `Grep` / `@`-mention paths under `reference/` without any `.cursorignore` negation.

**One-time setup on each clone** (append to `.git/info/exclude`):

```bash
cat >> .git/info/exclude <<'EOF'
# Local C++ reference trees — see reference/README.md
reference/classic-772/
reference/cipsoft-772/
reference/tvp-772/
reference/archives/
/reference/classic-772/client/tibia.pem
/reference/cipsoft-772/client/tibia.pem
/reference/classic-772/client/Tibia772*.exe
/reference/cipsoft-772/client/Tibia772*.exe
EOF
```

**code-review-graph** uses `git ls-files` on the main repo, so local reference checkouts are **not** in the default graph. To index C++ reference locally:

```bash
scripts/register_reference_graph.sh
```

That creates a nested local-only git repo inside each reference checkout (never pushed) and registers it with CRG. Agents then use `cross_repo_search_tool` for 772 C++ lookups (`ref-772-mechanics`, `ref-772-wire`).
