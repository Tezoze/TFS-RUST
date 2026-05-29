# Plan: Basic Config Manager (network) + Real DB Save Path

Status: in progress (P1–P5 done for config/save rollout; see §9) · Owner: Australis Rust port · Scope: minimal, idiomatic, behavior-compatible with TFS 1.4.2

---

## 0. Problem Statement

Two concrete gaps are blocking normal operation:

1. **Config manager is a thin Lua wrapper, not wired to networking.**
   `ConfigManager` (`crates/tfs-rust-core/src/config.rs`) can read arbitrary Lua globals, but `run_server::run` (`crates/tfs-rust-core/src/run_server.rs:178-184`) binds listeners using env vars (`TFS_LOGIN_ADDR`, `TFS_GAME_ADDR`, `TFS_GAME_PORT`) instead of reading `ip` / `loginProtocolPort` / `gameProtocolPort` / `statusProtocolPort` / `bindOnlyGlobalAddress` from `config.lua`. This is a regression from TFS C++ behavior (`src/configmanager.cpp:168-194`, `src/otserv.cpp:298-305`).

2. **No server-side save path.**
   `PlayerStore::save_player` (`crates/tfs-rust-db/src/player.rs:309`) is fully implemented — transactional, parity with C++ `IOLoginData::savePlayer` — but is **never called anywhere**. Players only persist if you take a raw DB snapshot. C++ calls it on logout (`IOLoginData::savePlayer` in `src/iologindata.cpp`) and via `g_game.saveGameState()` on server-save and shutdown (`src/game.cpp`).

The database *connection* is also fed from `DATABASE_URL` env only (`run_server.rs:93`) rather than from `mysqlHost` / `mysqlUser` / `mysqlPass` / `mysqlDatabase` / `mysqlPort` in `config.lua`. This needs the same config-driven treatment.

---

## 1. Compatibility Mandate (what must stay identical)

Per `TFS-Core.mdc` — behavior is specified by TFS 1.4.2 C++. For this change that means:

- **Key names & defaults** exactly match `src/configmanager.cpp`:
  - `ip` (default `"127.0.0.1"`)
  - `bindOnlyGlobalAddress` (default `false`)
  - `loginProtocolPort` (default `7171`)
  - `gameProtocolPort` (default `7172`)
  - `statusProtocolPort` (default `7171`)
  - `mysqlHost` (default `"127.0.0.1"`), `mysqlUser` (default `"forgottenserver"`), `mysqlPass` (default `""`), `mysqlDatabase` (default `"forgottenserver"`), `mysqlPort` (default `3306`), `mysqlSock` (default `""`)
- **Bind semantics** match C++ `ServiceManager::add`:
  - If `bindOnlyGlobalAddress == false` → bind `0.0.0.0:<port>` (TFS C++ default behavior: listen on all interfaces).
  - If `bindOnlyGlobalAddress == true` → bind `<ip>:<port>` only.
  - `loginProtocolPort` and `statusProtocolPort` may share the same port (default: both `7171`); when equal, bind **one** listener that multiplexes login + status (matches C++ `ProtocolLogin` + `ProtocolStatus` + `ProtocolOld` sharing `LOGIN_PORT` via `ServiceManager`, see `src/otserv.cpp:299-305`).
- **Save semantics** match C++ `IOLoginData::savePlayer`: transactional, with the `player.save == 0` fast-path already encoded at `player.rs:310-313`.
- **Save trigger points** match C++:
  - On logout (clean disconnect) — `Game::playerLogout` → `IOLoginData::savePlayer`.
  - On server save / shutdown — `Game::saveGameState` iterates all online players.
  - (Optional, C++ has it) periodic auto-save; not in scope for v1.

Everything *internal* to how we organize the Rust code is free to be idiomatic. No `Arc<RwLock<T>>`; we use existing ownership (`GameWorld` already owns `ConfigManager` as `Arc<ConfigManager>`).

---

## 2. Target Design

### 2.1 Typed `NetConfig` + `DbConfig` views over `ConfigManager`

Add two small, pure view structs in `crates/tfs-rust-core/src/config.rs`. Each is populated once at startup by calling the existing `get_string` / `get_i64` / `get_bool` helpers, applying the C++ defaults when keys are missing.

```rust
// C++ ref: src/configmanager.cpp:168-194
pub struct NetConfig {
    pub ip: String,                   // "ip"
    pub bind_only_global_address: bool, // "bindOnlyGlobalAddress"
    pub login_port: u16,              // "loginProtocolPort"
    pub game_port: u16,               // "gameProtocolPort"
    pub status_port: u16,             // "statusProtocolPort"
}

// C++ ref: src/configmanager.cpp:178-184
pub struct DbConfig {
    pub host: String,
    pub user: String,
    pub pass: String,
    pub database: String,
    pub port: u16,
    pub sock: String,  // unix socket; empty = TCP
}
```

Each gets a `from_config(&ConfigManager) -> Result<Self>` constructor that:

- Calls the typed getter.
- On `TfsRustError::Config("missing ...")` for a **defaultable** key, substitutes the C++ default.
- On type mismatch (e.g., `ip = 7171`), returns a hard error — do **not** silently coerce. This is stricter than C++ but safer; if it causes friction we relax to match C++ (which coerces via `lua_tostring`).

Helper to centralize the "missing key → default" pattern:

```rust
fn get_string_or(cfg: &ConfigManager, key: &str, default: &str) -> Result<String> {
    match cfg.get_string(key) {
        Ok(v) => Ok(v),
        Err(TfsRustError::Config(msg)) if msg.starts_with("missing") => Ok(default.into()),
        Err(e) => Err(e),
    }
}
// analogous get_i64_or, get_bool_or
```

Port values: read as `i64`, range-check to `u16`, error if out of range.

### 2.2 Env → Config migration in `run_server::run`

Replace the env-driven binding block at `run_server.rs:178-184` with config-driven binding.

**Precedence (kept pragmatic):** env var, if present, still wins over `config.lua`. This preserves current Docker / test workflows. Order:

1. `TFS_LOGIN_ADDR` / `TFS_GAME_ADDR` if set → use as-is (current behavior).
2. Else construct from `NetConfig`:
   - `bind_host = if net.bind_only_global_address { &net.ip } else { "0.0.0.0" }`
   - `login_addr = format!("{bind_host}:{}", net.login_port)`
   - `game_addr  = format!("{bind_host}:{}", net.game_port)`
3. `TFS_GAME_PORT` / `TFS_PUBLIC_IP` behave the same way, falling back to `net.game_port` / `net.ip`.

Status port (new): if `net.status_port == net.login_port`, reuse the login listener (matches C++). Otherwise bind a third listener. **v1 scope**: only wire it if it differs; we don't yet have a separate `ProtocolStatus` handler, so log a warning and skip when different. Real `ProtocolStatus` is tracked separately.

### 2.3 DB URL built from `DbConfig`

Replace `run_server.rs:93-97` with:

```rust
let database_url = match std::env::var("DATABASE_URL") {
    Ok(u) => u,  // env override wins
    Err(_) => {
        let d = DbConfig::from_config(&cfg)?;
        // mysql://user:pass@host:port/db  (URL-encode user/pass/db)
        format!(
            "mysql://{}:{}@{}:{}/{}",
            url_encode(&d.user), url_encode(&d.pass),
            d.host, d.port, url_encode(&d.database),
        )
    }
};
```

Use `percent-encoding` (already transitive in the workspace) for credentials. Unix socket (`mysqlSock`) is **out of scope for v1** — document as a known gap; sqlx supports it but requires a different URL form.

Connection pool sizing stays as the current `1..=5`; C++ `mysqlConnectionPool*` keys can be wired later.

### 2.4 Save path wiring (the important one)

Goal: calling `PlayerStore::save_player` at two points on a live server.

#### 2.4.1 On logout

Logout currently flows through `run_game_loop` (or `login.rs` for protocol-level disconnect). The game loop owns `GameWorld`, which owns the `DbPool`. The save payload `PlayerSaveData` is built from the in-memory `Player` (fields already exist on `CreatureKind::Player`).

Steps:

1. Add `GameWorld::build_player_save_data(&self, cid: CreatureId) -> Result<PlayerSaveData>` — pure read of in-memory state. C++ ref: `src/iologindata.cpp` (the portion of `savePlayer` that populates the row).
2. In `run_game_loop`, on the `GameCommand::Logout` / disconnect branch (or equivalent; locate current handler), before removing the creature:
   - Build `PlayerSaveData`.
   - `tokio::spawn` a save task: `let db = world.db.clone(); let data = ...; spawn(async move { PlayerStore::new(&db).save_player(&data).await })`.
   - Log errors; do **not** block the game loop on DB I/O (per project threading rules — move expensive I/O off the sim loop).
3. Remove the creature from `GameWorld` *after* enqueuing the save (the save owns a cloned `PlayerSaveData`, not a borrow).

**C++ parity note:** C++ saves synchronously inside the dispatcher thread. Rust spawns it; observable DB end-state is identical. No deviation flag required — "idiomatic Rust latitude" under Compatibility Mandate.

#### 2.4.2 On shutdown / server save

Add a shutdown hook in `run_server::run`:

1. Install a `tokio::signal::ctrl_c` handler (or reuse existing one if present) that sets a shutdown flag on `GameWorld`.
2. The game loop, on seeing shutdown, iterates all online players and awaits `save_player` for each (bounded concurrency via `futures::stream::iter(...).buffer_unordered(8)`).
3. Only after all saves complete does the loop exit and the process returns.

This replaces the snapshot-only workflow.

#### 2.4.3 Periodic save (optional, flagged out of v1)

C++ does not run a periodic save by default in 1.4.2 (server-save is a scheduled global event). We skip this in v1 and document as a follow-up.

---

## 3. File-by-File Changes

| File | Change |
|------|--------|
| `crates/tfs-rust-core/src/config.rs` | Add `NetConfig`, `DbConfig`, `from_config`, defaults helpers. No change to existing `ConfigManager` API. |
| `crates/tfs-rust-core/src/run_server.rs` | Replace env-only bind block (`:178-184`) with config-driven logic + env override. Build DB URL from `DbConfig` when `DATABASE_URL` unset. Install shutdown hook. |
| `crates/tfs-rust-core/src/game_loop.rs` (or wherever logout is handled) | On player-leave, build `PlayerSaveData`, spawn save task. On shutdown, flush all online players. |
| `crates/tfs-rust-core/src/game_world_save.rs` | `GameWorld::build_player_save_data` + item-tree flatten (C++ `saveItems`). |
| `crates/tfs-rust-core/src/creature/player.rs` | `PlayerPersistBaseline` + `Player::persist` (login snapshot for save). |
| `crates/tfs-rust-core/src/item_blob.rs` | `write_item_blob` (`Item::serializeAttr` parity). |
| `tasks/lessons.md` | Capture any Rust-specific lesson (e.g., LocalSet + `tokio::spawn` for `Send` save task). |

Nothing in `tfs-rust-db` needs to change — `PlayerStore::save_player` is already correct.

---

## 4. Tests

Minimum set (co-located with each change):

1. **`config.rs` unit tests** (no DB, no network):
   - `NetConfig::from_config` returns C++ defaults when keys absent.
   - Reads `ip` / `loginProtocolPort` / `gameProtocolPort` / `statusProtocolPort` / `bindOnlyGlobalAddress` correctly from a minimal inline `config.lua` string.
   - Port out-of-range (`0` or `>65535`) returns error.
   - `DbConfig::from_config` same coverage.
2. **`run_server` bind logic** — extract the "compute bind addr from NetConfig + env" into a pure function; unit-test the four combinations (env set / unset × `bindOnlyGlobalAddress` true / false).
3. **Save-on-logout integration test** (behind `#[ignore]` unless `DATABASE_URL` is set):
   - Load a test character, mutate a field (e.g., `experience += 1`), trigger logout, reconnect, assert the change persisted. Mirrors C++ `IOLoginData::savePlayer` contract.
4. **Save-on-shutdown test** — same, but triggered via the shutdown path.

No existing tests should be weakened or removed.

---

## 5. Rollout Steps (execution order)

1. ✅ Landed `NetConfig` + `DbConfig` + unit tests. No runtime wiring yet. (Pure addition, zero risk.)
2. ✅ Wired `run_server::run` to use typed config (`NetConfig`/`DbConfig`) with env overrides preserved.
3. ✅ `build_player_save_data` (`game_world_save.rs`) + `PlayerPersistBaseline` at login + `tokio::spawn` save on `GameCommand::PlayerDisconnect` (`game_loop.rs`). Manually verify: log in, walk, log out, inspect DB row changes without snapshot.
4. ✅ **P4** — SIGINT → `GameCommand::Shutdown` → `flush_online_players_to_db` in `game_loop.rs` (bounded `JoinSet` concurrency 8, awaited), then `run_server` aborts both TCP acceptor tasks. Manually verify: log in, SIGINT, DB shows final state.
5. ✅ **P5** — `tasks/lessons.md`: lesson **4** (shutdown / `JoinSet` / acceptor `spawn` ordering) and lesson **5** (config.lua vs `DATABASE_URL` / `TFS_*` env overrides). Closes the “capture lessons” rollout item.

Each step ends with `cargo check -p tfs-rust-core` + `cargo test -p tfs-rust-core`.

---

## 6. Explicit Non-Goals (v1)

- Connection pool tuning from `mysqlConnectionPool*` keys.
- Unix socket (`mysqlSock`) support.
- Separate `statusProtocolPort` listener when it differs from `loginProtocolPort`.
- Periodic auto-save timer.
- Hot reload of `config.lua` at runtime (C++ `ConfigManager::reload`). Restart is required, matching 95% of C++ operator practice.

---

## 7. Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Config key typo silently uses default → wrong port | Log every defaulted key at INFO so operators see it in startup output. |
| Save task panics / DB down at logout → data loss | `save_player` already returns `Result`; log at ERROR and, on shutdown path, await and surface the first error before exit. |
| Save contention on shutdown (many players) | `buffer_unordered(8)` caps concurrency. |
| Env-var precedence surprises users | Log which source each value came from (`env` vs `config.lua` vs `default`). |

---

## 8. C++ References (for traceability)

- `src/configmanager.cpp:168-194` — key names and defaults for ip/ports/bindOnlyGlobalAddress.
- `src/configmanager.cpp:178-184` — MySQL key names and defaults.
- `src/otserv.cpp:185-192` — DB connect order at startup.
- `src/otserv.cpp:298-305` — `ProtocolGame` / `ProtocolLogin` / `ProtocolStatus` / `ProtocolOld` port binding.
- `src/iologindata.cpp` (`IOLoginData::savePlayer`) — save contract (already ported to `crates/tfs-rust-db/src/player.rs:309`).
- `src/game.cpp` (`Game::saveGameState`, `Game::playerLogout`) — save trigger points.

Every new function must carry a matching `// C++ ref: ...` comment.

---

## 9. Post-P3 follow-ups (tracked)

| Item | Status / notes |
|------|-----------------|
| **P4 — shutdown / SIGINT flush** | **Done (April 2026).** `wait_for_shutdown_signal` → `cmd_tx.send(Shutdown)`; `run_game_loop` handles `GameCommand::Shutdown` with `flush_online_players_to_db` (`tokio::task::JoinSet`, max 8 in flight — same idea as `buffer_unordered(8)`). `graceful_shutdown` is a no-op with docs pointing to the game loop. Both TCP acceptors are `tokio::spawn`’d and **aborted** after the local game task finishes. |
| **`last_depot_id` / `skip_depot_save`** | **Open.** `PlayerPersistBaseline::last_depot_id` is still `-1` until depot is represented in the sim. When depot UI mutates, set from runtime (C++ `Player::lastDepotId`) so `skip_depot_save` matches C++ `savePlayer`. |
| **`GameWorld::player_logout`** | **Open.** Not wired from `run_game_loop`; exit is `PlayerDisconnect` only. Unify to one helper if `player_logout` becomes a second path, to avoid double-remove / missed save. |
| **`tasks/lessons.md` (P5)** | **Done.** Lessons 4–5 in `tasks/lessons.md` cover shutdown flush vs disconnect `spawn` and config vs env precedence. |
| **Integration tests (plan §4)** | **Open.** `#[ignore]` save-on-logout / save-on-shutdown behind `DATABASE_URL` still recommended. |
