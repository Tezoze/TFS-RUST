# TFS Rust

[![Build Status](https://github.com/Tezoze/TFS-RUST/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/Tezoze/TFS-RUST/actions/workflows/rust.yml)

Rust rewrite of **The Forgotten Server 1.4.2** (TFS C++ in repo-root `src/` is the behavioral reference). The goal is **1:1 parity** with TFS 1.4.2 mechanics, database schema, and Lua scripting, with a modern architecture: Tokio for I/O, a single-threaded game simulation, and generational entity storage via `slotmap`.

**Default target today:** OTClient v8, Tibia protocol **10.98**. Multi-version wire support (e.g. 7.72) is in progress — see [Protocol versioning](docs/PROTOCOL_VERSIONING.md).

Use a **custom OTClient v8** build aligned with this server’s protocol expectations.

---

## Architecture

| Layer | Crate | Role |
|-------|--------|------|
| Simulation | `tfs-rust-core` | `GameWorld`, map, creatures, items, combat hooks, Lua events — **game thread only** |
| Networking | `tfs-rust-net` | TCP, RSA/XTEA, packet parse/encode, version-aware **codec** seam |
| Database | `tfs-rust-db` | MariaDB via SQLx (prepared statements, migrations) |
| Content | `tfs-rust-content` | OTB, OTBM, `items.xml`, monsters, vocations |
| Scripting | `tfs-rust-lua` | LuaJIT (`mlua`) bridge to TFS-style APIs |
| Shared | `tfs-rust-common` | IDs, positions, opcodes, `ProtocolVersion` / `ProtocolCaps` |

I/O threads parse packets and run DB queries; the game thread owns all world state and communicates over `mpsc` channels (`GameCommand` in, encoded packets out). See [Protocol versioning](docs/PROTOCOL_VERSIONING.md) for how wire format is separated from mechanics.

---

## Quick start

1. **Build and run** — follow [docs/COMPILING.md](docs/COMPILING.md) (requirements, `cargo build`, `config.lua`, MariaDB, `scripts/run_server.sh`).
2. Copy `config.lua.dist` → `config.lua` and set `clientVersion = 1098` (and MySQL credentials).
3. Ensure `data/`, `key.pem`, and your OTBM map path (`TFS_DATA_DIR` / `TFS_MAP_OTBM`) are in place.

```bash
cargo build --release --bin tfs-rust
cp config.lua.dist config.lua
./scripts/run_server.sh
```

Login **7171**, game **7172** by default.

---

## Documentation

| Doc | Contents |
|-----|----------|
| [docs/COMPILING.md](docs/COMPILING.md) | Build, test, first-time DB and server setup |
| [docs/PROJECT_STATUS.md](docs/PROJECT_STATUS.md) | What works today vs still open |
| [docs/PROTOCOL_VERSIONING.md](docs/PROTOCOL_VERSIONING.md) | 7.72 vs 10.98 wire/mechanics plan (Track A/B) |
| [docs/OTCLIENT_INFO.md](docs/OTCLIENT_INFO.md) | OTCv8 protocol quirks vs vanilla TFS |

Legacy C++ reference trees (`gameserver/`, `tibia-game-master/`) are local-only (gitignored) for 7.72 porting — not required to run 10.98.

---

## Workspace layout

```
crates/tfs-rust-{common,content,db,net,lua,core}/   # Rust server
rust-src/main.rs                                    # `tfs-rust` binary entry
data/                                               # Lua scripts, XML, map assets
src/                                                # TFS 1.4.2 C++ (reference, not built by Cargo)
tools/packet-proxy/                                 # Optional packet capture helper
```

---

## Contributing

- Match TFS 1.4.2 behavior unless explicitly documented; cite C++ file + function in ported Rust.
- Run before a PR: `SQLX_OFFLINE=true cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all`.
- Use [GitHub Issues](https://github.com/Tezoze/TFS-RUST/issues) for bugs and features (not general support threads).

---

## License

Same lineage as The Forgotten Server — see repository history and `LICENSE` if present. Third-party assets under `data/` follow their original Tibia/OT community terms.
