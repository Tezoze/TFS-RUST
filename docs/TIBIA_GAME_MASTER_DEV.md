# tibia-game-master — local compile & debug

The CipSoft 7.72 decompile lives at `reference/cipsoft-772/tibia-game-master/` (gitignored). Use it as the **772 mechanics reference** when validating TFS-RUST parity — not for wire/packets. TVP wire reference: `reference/tvp-772/gameserver/`. See [reference/README.md](../reference/README.md).

## What you need

| Piece | Purpose | Status in this repo |
|-------|---------|---------------------|
| `reference/cipsoft-772/tibia-game-master/` | Game server source | Present (gitignored) |
| OpenSSL `libcrypto` | RSA (`apt install libssl-dev` / `pacman -S openssl`) | System package |
| **Game data tarball** | `.tibia`, `map/`, `dat/`, `usr/`, scripts | `reference/archives/tibia-game.tarball.tar.gz` |
| `reference/cipsoft-772/tibia-querymanager/` | Hard runtime dependency (DB) | Cloned alongside game source |
| `reference/cipsoft-772/tibia-login/` | Character list / login (port 7171) | Cloned alongside game source |

## Build (verified)

```bash
scripts/tibia_game_dev.sh check
scripts/tibia_game_dev.sh build
```

This builds `tibia-game-master/build/game` with:

- `DEBUG=1` — `-g -Og -DENABLE_ASSERTIONS=1` for gdb and asserts
- `TIBIA772=1` (default) — 7.72 protocol flag (`-DTIBIA772=1` in `communication.cc`)

Manual equivalent:

```bash
cd reference/cipsoft-772/tibia-game-master
make -B DEBUG=1 -j$(nproc) \
  CFLAGS="-m64 -fno-strict-aliasing -pedantic -Wall -Wextra \
    -Wno-deprecated-declarations -Wno-unused-parameter -Wno-format-truncation \
    -std=c++11 -pthread -DOS_LINUX=1 -DARCH_X64=1 -DTIBIA772=1 \
    -g -Og -DENABLE_ASSERTIONS=1"
```

## Run stack

The game server **will not start** without:

1. A `.tibia` config file in the **working directory** (paths inside point at data dirs)
2. A running **query manager** on the host/port/password configured in `.tibia`

### 1. Clone supporting services (once)

```bash
git clone https://github.com/fusion32/tibia-querymanager.git reference/cipsoft-772/tibia-querymanager
git clone https://github.com/fusion32/tibia-login.git reference/cipsoft-772/tibia-login
scripts/tibia_game_dev.sh build
```

Query manager uses embedded SQLite — no MariaDB required for local dev. Default port **7173**, password **`a6glaf0c`** (`config.cfg.dist`).

Login server listens on **7171** and talks to the query manager at `127.0.0.1:7173`. With `TIBIA772=1` (default in the dev script), it accepts 7.72 clients.

Optional sample accounts: copy `tibia-querymanager/sqlite/z-999-initial-data.sql` → `sqlite/patches/` so QM creates test chars on first boot.

### 2. Prepare game data

Extract into `reference/cipsoft-772/runtime/` (or repo root for legacy layout):

```bash
mkdir -p reference/cipsoft-772/runtime
tar -xzf reference/archives/tibia-game.tarball.tar.gz -C reference/cipsoft-772/runtime --no-same-owner

scripts/tibia_game_dev.sh setup
```

`setup` auto-detects game data at `reference/cipsoft-772/runtime/`, patches `.tibia` paths, installs the query-manager test-account patch, and copies the debug `game` binary + `tibia.pem`.

**First-run cleanup** (from upstream README):

- Delete stale `save/game.pid` if the server did not exit cleanly
- Clear or replace stale `dat/owners.dat` if QM errors on missing characters
- Use fresh `origmap` → `map` for a clean world

### 3. Start services

**One command** (background, logs under `reference/cipsoft-772/state/.tibia-cipsoft/`):

```bash
scripts/tibia_game_online.sh start
scripts/tibia_game_online.sh status
scripts/tibia_game_online.sh stop
```

Import Crowoo on first boot:

```bash
TIBIA_IMPORT_CHARACTER=Crowoo scripts/tibia_game_online.sh start
```

**Manual** (three terminals):

Terminal 1 — query manager:

```bash
scripts/tibia_game_dev.sh run-qm
```

Terminal 2 — login server:

```bash
scripts/tibia_game_dev.sh run-login
```

Terminal 3 — game server (foreground, no daemon fork):

```bash
export TIBIA_GAME_DATA=/path/to/tibia-game
scripts/tibia_game_dev.sh run-game
```

Pass `nofork` so the process stays in the shell (required for gdb).

## Client login

You need a **Tibia 7.72 client** and an **IP changer** pointed at your machine.

| Setting | Value |
|---------|-------|
| Login IP | `127.0.0.1` |
| Login port | `7171` |
| Game port | `7172` (world *Zanera* — from query manager) |
| Account number | `111111` |
| Password | `tibia` |
| Characters | `Gamemaster`, `Player` |

772 login uses the numeric **account number**, not an email or character name.

If the client says **"Your terminal version is too old"**, the login server was built for **7.7 (770)** while your client sends **772**. Rebuild and restart:

```bash
scripts/tibia_game_dev.sh build-login
scripts/tibia_game_online.sh restart
```

The `.usr` files in the leaked tarball (e.g. `usr/22/13157122.usr`) are **not** login-ready by themselves — the query manager must have matching `Accounts` / `Characters` rows. For local dev, use the sample account above (`setup` installs it). Import a leaked character:

```bash
scripts/tibia_game_dev.sh list-characters --level 50 --near 32345,32225,7
scripts/tibia_game_dev.sh import-character Crowoo
# login: account 111111 / password tibia → select Crowoo
```

Use `--account N` to attach the character to a different query-manager account.

RSA key: **`reference/cipsoft-772/client/tibia.pem`**. Login and game read `tibia.pem` from their working directory; `setup` copies it into runtime. The login server gets a copy at `reference/cipsoft-772/tibia-login/tibia.pem` on start.

```bash
scripts/tibia_game_online.sh show-rsa   # print modulus for your active key
```

If the client was configured for a different key, login fails with `Failed to decrypt asymmetric data` in `login.log`.

### IP changer (772 client under Wine)

**`wine build/ipchanger.exe local` often fails on Linux** — it needs a running `TibiaClient` window (`FindWindow`), which Wine may not expose. Use the **on-disk patcher** instead:

Run from **repo root** (paths are relative to `/mnt/storage2/TFS_RUST`):

```bash
cd /mnt/storage2/TFS_RUST
python3 scripts/patch_tibia_client.py ~/Downloads/Tibia772/Tibia.exe \
  -o ~/Downloads/Tibia772/Tibia-local.exe
wine ~/Downloads/Tibia772/Tibia-local.exe
```

(`fish` does not run `scripts/foo.py` unless you `cd` to the repo or use `python3 /full/path/...`.)

Patches login hosts → `127.0.0.1`, RSA from `reference/cipsoft-772/client/tibia.pem`, keeps port 7171. Original backed up as `Tibia.exe.bak` when patching in place.

Memory-based ipchanger (only if Tibia is already running in the **same** `WINEPREFIX`):

```bash
scripts/build_ipchanger.sh
cd reference/cipsoft-772/tibia-ipchanger-master && wine build/ipchanger.exe local
```

### 4. gdb

```bash
scripts/tibia_game_dev.sh gdb
```

Useful breakpoints for 772 chase/dance parity:

- `TShortway::Calculate` / `TShortway::Expand` — `cract.cc`
- `TMonster::IdleStimulus` / chase todo — `crnonpl.cc`

## Live monster pathing trace (CipSoft ↔ Rust)

Both servers emit **matching JSONL** to `log/chase_path.log` for side-by-side parity work.

### Enable CipSoft (tibia-game-master)

Add to repo-root `.tibia` (or export env before start):

```
ChasePathDebug = 1
```

Or:

```bash
export TIBIA_CHASE_PATH_DEBUG=1
scripts/tibia_game_online.sh restart
```

Log: `{LOGPATH}/chase_path.log` (usually `log/chase_path.log` under game data).

### Enable TFS-RUST

```bash
export TFS_CHASE_PATH_DEBUG=1
# optional: export TFS_CHASE_PATH_LOG=/path/to/chase_path.log
cargo run -p tfs-rust-server   # or your usual server entrypoint
```

### Event types (same schema, `src` differs)

| `evt` | When |
|-------|------|
| `branch` | `IdleStimulus` chose a chase/dance/roam arm (`melee_chase`, `melee_dance`, `dist_chase`, …) |
| `todo_go` | `ToDoGo` entered / single-step / `NOWAY` |
| `shortway` | `TShortway::Calculate` result + queued world steps |
| `go_exec` | Monster actually moved one tile (`Go` / Rust walk step) |

Example line:

```json
{"src":"cip","evt":"shortway","tick":12345,"id":999,"name":"Rat","start":{"x":100,"y":100,"z":7},"dest":{"x":103,"y":102,"z":7},"rel_dest":{"x":3,"y":2},"visible":10,"min_wp":150,"must":0,"max":3,"ok":1,"steps":[{"x":101,"y":101,"z":7}]}
```

### Compare logs after a repro

```bash
python scripts/compare_chase_live_logs.py \
  --cip /mnt/storage2/TFS_RUST/log/chase_path.log \
  --rust ./log/chase_path.log \
  --monster Rat
```

Rebuild CipSoft after pulling chase debug: `scripts/tibia_game_dev.sh setup`.

Gap analysis from live log compare: [`TFS-RUST_772_Chase_Path_Parity_Gaps.md`](TFS-RUST_772_Chase_Path_Parity_Gaps.md).

## Pathfinding-only debugging (no full server)

For **TShortway chase path parity**, the Python harness reimplements CipSoft logic and compares against Rust:

```bash
python scripts/compare_chase_pathfinding.py --build-rust
```

This avoids the tarball + QM + login stack when you only need algorithm output diffs.

## Makefile flags (upstream)

From `tibia-game-master/README.md`:

| Flag | Effect |
|------|--------|
| `-DTIBIA772=1` | 7.72 protocol (default in our dev script) |
| `-DALLOW_LOCAL_PROXY=1` | Local proxy header for connections |
| `-DBIND_ACCEPTOR_TO_GAME_ADDRESS=1` | Bind game socket to configured address only |

## Related repos

- [tibia-game](https://github.com/fusion32/tibia-game) — same source as `tibia-game-master/`
- [tibia-querymanager](https://github.com/fusion32/tibia-querymanager)
- [tibia-login](https://github.com/fusion32/tibia-login)
- [tibia-web](https://github.com/fusion32/tibia-web) — account management (optional)
