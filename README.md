# TFS Rust

[![Build Status](https://github.com/Tezoze/TFS-RUST/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/Tezoze/TFS-RUST/actions/workflows/rust.yml)

Rust rewrite of the **7.72** reference server. The goal is exact observable parity with 7.72 wire protocol, mechanics, and outcomes, using a modern architecture: Tokio for I/O, a single-threaded game simulation, and generational entity storage via `slotmap`.

**Default target today:** Tibia protocol **7.72**. TFS 1.4.2 / protocol **10.98** support is planned but not the current focus.

Use a **7.72-compatible client** (or a custom OTClient aligned with this server’s protocol expectations).

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

I/O threads parse packets and run DB queries; the game thread owns all world state and communicates over `mpsc` channels (`GameCommand` in, encoded packets out). Wire format and mechanics are separated by `ProtocolVersion`/`ProtocolCodec` and `MechanicsProfile`; see `.cursor/rules/TFS-protocol-versioning.mdc` for the full split.

---

## Quick start

1. **Build and run** — follow [docs/COMPILING.md](docs/COMPILING.md) (requirements, `cargo build`, `config.lua`, MariaDB, `scripts/run_server.sh`).
2. Copy `config.lua.dist` → `config.lua` and set `clientVersion = 772` (and MySQL credentials).
3. Ensure `data/`, `key.pem`, and your OTBM map path (`TFS_DATA_DIR` / `TFS_MAP_OTBM`) are in place.

```bash
cargo build --release --bin tfs-rust
cp config.lua.dist config.lua
./scripts/run_server.sh
```

Login **7171**, game **7172** by default.

**Docker** — pull the image published from `main` (`ghcr.io/tezoze/tfs-rust`), or build locally:

```bash
cp .env.example .env   # optional
docker compose pull && docker compose up
# or, compile in Docker: docker compose up --build
```

Client: login port **7171**, account **`1`**, password **`1`**. Characters: **God**, **Master Sorcerer**, **Elder Druid**, **Royal Paladin**, **Elite Knight**.

See [docs/DOCKER.md](docs/DOCKER.md).

---

## Documentation

| Doc | Contents |
|-----|----------|
| [docs/COMPILING.md](docs/COMPILING.md) | Build, test, first-time DB and server setup |
| [docs/DOCKER.md](docs/DOCKER.md) | Compose, GHCR image, ports, seeded account |
| [docs/772_OTCLIENT_PARITY.md](docs/772_OTCLIENT_PARITY.md) | 7.72 OTClient protocol parity notes |
| [docs/WALK_772_PARITY_AUDIT.md](docs/WALK_772_PARITY_AUDIT.md) | 7.72 walk / movement parity |
| [docs/772_MONSTER_AI_AUDIT.md](docs/772_MONSTER_AI_AUDIT.md) | 7.72 monster AI and chase parity |
| [docs/OTCLIENT_INFO.md](docs/OTCLIENT_INFO.md) | OTClient protocol quirks (legacy reference) |
| [reference/README.md](reference/README.md) | Local 772 C++ reference tree layout |

Local C++ reference trees for 7.72 live under **`reference/`** (gitignored). See [reference/README.md](reference/README.md) for `tvp-772/gameserver/src/` (wire authority) and `classic-772/tibia-game-master/src/` (mechanics authority). The repo-root `src/` is the optional TFS 1.4.2 (1098) reference tree, not built by Cargo.

---

## Workspace layout

```
crates/tfs-rust-{common,content,db,net,lua,core}/   # Rust server
rust-src/main.rs                                    # `tfs-rust` binary entry
data/                                               # 7.72 Lua scripts, XML, map assets
lua-defs/                                           # Generated LuaLS stubs (`emit-lua-defs`; not executed)
data1098/                                           # 10.98 data (future target)
src/                                                # Optional TFS 1.4.2 C++ (1098 reference, not built by Cargo)
reference/                                          # Local 7.72 C++ reference trees (gitignored; see reference/README.md)
tools/packet-proxy/                                 # Optional packet capture helper
```

---

## Contributing

- Match 7.72 reference behavior unless explicitly documented; cite C++ file + function in ported Rust.
- Run before a PR: `SQLX_OFFLINE=true cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all`.
- Use [GitHub Issues](https://github.com/Tezoze/TFS-RUST/issues) for bugs and features (not general support threads).

---

## License

Same lineage as The Forgotten Server — see repository history and `LICENSE` if present. Third-party assets under `data/` follow their original Tibia/OT community terms.
