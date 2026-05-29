# Compiling TFS Rust

Guide to building and running the server from source on Linux, macOS, and Windows.

**Binary name:** `tfs-rust`  
**Default ports:** login `7171`, game `7172` (from `config.lua`)  
**Client:** OTClient v8, protocol **10.98**

---

## 1. Requirements

### Required

| Tool | Notes |
|------|--------|
| **Rust (stable)** | Install via [rustup](https://rustup.rs/) — `rustup default stable` |
| **C toolchain** | `gcc`/`clang` + `make` — needed to build vendored **LuaJIT** (`mlua`) |
| **Git** | Clone the repository |
| **MariaDB or MySQL** | Runtime only — for accounts, characters, save/load |

### Optional (development)

| Tool | Purpose |
|------|---------|
| `rustfmt`, `clippy` | `rustup component add rustfmt clippy` |
| `cargo-sqlx` CLI | Regenerate `.sqlx/` offline query cache after schema/query changes |
| `psmisc` (`fuser`) | `scripts/run_server.sh` frees ports 7171/7172 before start (Linux) |

### Not required to compile

- The legacy **C++** tree (`src/`, `CMakeLists.txt`) — reference only; Rust does not build it
- The root **`Dockerfile`** — still targets the old C++ `tfs` binary; use native Rust build for now
- A live database — if `.sqlx/` is present, set `SQLX_OFFLINE=true` to compile without MariaDB

---

## 2. Clone

```bash
git clone https://github.com/Tezoze/TFS-RUST.git
cd TFS-RUST
```

`key.pem` (OT protocol RSA key) is included in the repo — the server expects it at the project root unless you set `TFS_RSA_PEM`.

---

## 3. System dependencies

### Arch Linux / CachyOS

```bash
sudo pacman -S base-devel git mariadb
```

Start MariaDB if needed:

```bash
sudo systemctl enable --now mariadb
```

### Debian / Ubuntu

```bash
sudo apt update
sudo apt install build-essential git pkg-config libssl-dev mariadb-server
```

### Fedora

```bash
sudo dnf install gcc gcc-c++ make git mariadb-server
```

### macOS

```bash
xcode-select --install
brew install mariadb
```

### Windows

1. Install [Rust](https://rustup.rs/) and **Visual Studio Build Tools** (C++ workload), or use **MSVC** + `link.exe`
2. Install [MariaDB](https://mariadb.org/download/) or MySQL
3. Build from **Developer PowerShell** or a shell where `cl.exe` is on `PATH`

LuaJIT is built from source via `mlua`; a working C compiler is mandatory on all platforms.

---

## 4. Build

From the repository root:

### Debug (fast compile, slower runtime)

```bash
cargo build --bin tfs-rust
```

Output: `target/debug/tfs-rust`

### Release (recommended for running a server)

```bash
cargo build --release --bin tfs-rust
```

Output: `target/release/tfs-rust`

### Compile without a database

CI and fresh clones use the committed SQLx offline cache:

```bash
SQLX_OFFLINE=true cargo build --release --bin tfs-rust
```

### Full workspace (all crates + tools)

```bash
cargo build --workspace --release
```

First build can take several minutes (LuaJIT + dependencies).

---

## 5. Verify the build

```bash
# Same as CI
SQLX_OFFLINE=true cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

---

## 6. First-time server setup

### 6.1 Configuration

```bash
cp config.lua.dist config.lua
```

Edit `config.lua` — at minimum set MySQL keys:

```lua
mysqlHost = "127.0.0.1"
mysqlUser = "forgottenserver"
mysqlPass = "your_password"
mysqlDatabase = "forgottenserver"
mysqlPort = 3306
```

`config.lua` is **gitignored**; never commit real passwords.

**Environment overrides** (optional):

| Variable | Effect |
|----------|--------|
| `DATABASE_URL` | Overrides `mysql*` keys in `config.lua` |
| `TFS_CONFIG` | Path to config file (default `config.lua`) |
| `TFS_RSA_PEM` | Path to PKCS#1 RSA PEM (default `key.pem` at repo root) |
| `TFS_DATA_DIR` | Datapack directory (default `data`) |
| `TFS_LOGIN_ADDR` / `TFS_GAME_ADDR` | Bind addresses |
| `TFS_GAME_PORT` / `TFS_PUBLIC_IP` | What clients are told for the game server |
| `RUST_LOG` | Log filter, e.g. `info,tfs_rust_core=debug` |

### 6.2 Database

Create database and user (example — adjust names/passwords):

```sql
CREATE DATABASE forgottenserver CHARACTER SET utf8mb4;
CREATE USER 'forgottenserver'@'localhost' IDENTIFIED BY 'your_password';
GRANT ALL PRIVILEGES ON forgottenserver.* TO 'forgottenserver'@'localhost';
FLUSH PRIVILEGES;
```

Import the baseline schema:

```bash
mysql -u forgottenserver -p forgottenserver < schema.sql
```

SQLx migrations (if you use the Rust migration runner on startup):

```bash
# Optional: install sqlx-cli
cargo install sqlx-cli --no-default-features --features mysql,rustls

export DATABASE_URL='mysql://forgottenserver:your_password@127.0.0.1:3306/forgottenserver'
sqlx migrate run --source crates/tfs-rust-db/migrations
```

Create at least one account and character (same as classic TFS / your AAC export).

### 6.3 Datapack & map

The repo includes `data/` (scripts, monsters, NPCs, XML) and `data/world/forgotten.otbm`. Run the server from the **repository root** so relative paths resolve.

---

## 7. Run

### Quick start (debug build via Cargo)

```bash
./scripts/run_server.sh
```

Equivalent to `cargo run --bin tfs-rust` from the repo root.

### Release binary directly

```bash
./target/release/tfs-rust
```

### Connect

1. Use **OTClient v8** configured for protocol **10.98**
2. Host: `127.0.0.1` (match `ip` in `config.lua` — avoid `localhost` if you bind IPv4-only)
3. Login port: `7171`, game port: `7172` (defaults)

See `docs/phase1-otclient-checkpoint.md` for manual connection troubleshooting.

---

## 8. Regenerating SQLx offline data

Only needed if you change `query!` macros or DB schema used at compile time:

```bash
export DATABASE_URL='mysql://USER:PASS@HOST:PORT/DATABASE'
cargo sqlx prepare --workspace -- --workspace
```

Commit updated files under `.sqlx/`. CI sets `SQLX_OFFLINE=true` so builds do not need a running DB.

---

## 9. Troubleshooting

### `no RSA PEM found`

Place `key.pem` at the repo root or set:

```bash
export TFS_RSA_PEM=/path/to/key.pem
```

### LuaJIT / C compile errors

Ensure `gcc`/`clang` and `make` are installed. On Windows, use the MSVC developer environment.

### `failed to connect to database`

- Check MariaDB is running: `systemctl status mariadb`
- Test: `mysql -u forgottenserver -p -h 127.0.0.1 forgottenserver`
- Or set `DATABASE_URL` explicitly:

```bash
export DATABASE_URL='mysql://user:pass@127.0.0.1:3306/forgottenserver'
```

### Port already in use (7171 / 7172)

```bash
ss -tlnp | grep -E '7171|7172'
# or use fuser (Linux):
fuser -k -n tcp 7171
fuser -k -n tcp 7172
```

### OTClient times out

- Server bound to `127.0.0.1` → client must use `127.0.0.1`, not `localhost` (IPv6 mismatch)
- WSL: Windows `127.0.0.1` ≠ Linux listener — use WSL IP or run client on same OS

### Slow or OOM first build

Release + LuaJIT compile is heavy. Use `cargo build --release -j $(nproc)` and ensure swap is available.

---

## 10. Related docs

| Doc | Contents |
|-----|----------|
| `docs/PROJECT_STATUS.md` | Feature status and architecture |
| `config.lua.dist` | All config keys and defaults |
| `scripts/run_server.sh` | Dev launcher |
| `README.md` | Project overview |
