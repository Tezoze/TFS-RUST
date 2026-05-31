# TFS Rust — Project Status

**Last updated:** 2026-06-01  
**Reference:** TFS 1.4.2 C++ (`src/`) is the source of truth for behavior.  
**Target client:** OTClient v8, Tibia protocol **10.98** (default). Wire codec is version-selectable — **7.72** is connectable via `Codec772` (Track A complete; mechanics still 1098 until Track B).

---

## Executive summary

TFS Rust is a ground-up rewrite of The Forgotten Server 1.4.2 into idiomatic Rust. The **foundation is in place**: async networking, login, map loading, player walking, item/container/inventory runtime, partial Lua scripting, and config-driven server startup with player save on logout/shutdown.

The server is **playable for connect → login → walk → move/equip items**, but is **not yet a full live shard**. Monster/NPC AI, combat dispatch, chat, spells, trade, and most Lua script surface area are still open work.

Rough scale today: **~140 Rust source files**, **~27k lines** across 6 workspace crates (plus a packet-proxy tool).

---

## What we're utilizing

### Language & toolchain

| Piece | Role |
|-------|------|
| **Rust 2021** | Primary language (workspace root; edition 2024 planned per project rules) |
| **Cargo workspace** | 6 crates + `tools/packet-proxy` |
| **GitHub Actions** | `cargo fmt`, `clippy -D warnings`, `cargo test --workspace` with `SQLX_OFFLINE=true` |

### Runtime & concurrency

| Crate / pattern | Role |
|-----------------|------|
| **Tokio** | Async TCP accept, packet I/O, DB queries, graceful shutdown |
| **Single-threaded game loop** | All simulation state on one thread (`GameWorld`, `SlotMap`s, `Map`) |
| **`mpsc` channels** | I/O threads → game thread (`GameCommand`); game thread → output queue |
| **`tokio::select!` (biased)** | Game tick (~50 ms), walk-wake, command recv, save completion |
| **`LocalSet` + `spawn_local`** | Game thread stays non-`Send`; no locks on world state |

### Data structures & safety

| Crate / pattern | Role |
|-----------------|------|
| **`slotmap::SlotMap`** | Generational storage for creatures and items (`CreatureId`, `ItemId`) |
| **`Cylinder` enum dispatch** | Tile / Container / Inventory — zero-cost alternative to C++ virtual cylinders |
| **`bitflags`** | Item flags, cylinder flags, protocol flags |
| **`DashMap`** | Concurrent structures at I/O boundaries where needed |

### Networking & protocol

| Crate / pattern | Role |
|-----------------|------|
| **`tfs-rust-net`** | TCP server, XTEA, RSA (raw `modpow`, not PKCS#1 decrypt), Adler32 framing |
| **`ProtocolCodec` seam** | Version-keyed wire codec — `Codec1098` (10.98/OTCv8) + `Codec772` (7.72, `gameserver/src/` parity); zero-cost `Codec` enum dispatch, caps-gated transport/login |
| **`bytes` / manual parsing** | Zero-copy-friendly packet read/write |
| **60+ client→server opcodes** | Parsed into `GamePacket` enum |
| **OTCv8 detection** | OS + `"OTCv8"` probe stored per connection |
| **Item encoding** | Fluid map, animation phase, optional OTCv8 description string |

### Database

| Crate / pattern | Role |
|-----------------|------|
| **SQLx + MariaDB** | Async prepared statements; migrations in `crates/tfs-rust-db/migrations/` |
| **`.sqlx/` offline cache** | CI builds without a live DB (`SQLX_OFFLINE=true`) |
| **`PlayerStore::save_player`** | Parity with C++ `IOLoginData::savePlayer`; called on logout + SIGINT shutdown |
| **`DbConfig` / `DATABASE_URL`** | URL from `config.lua` mysql* keys, env override when set |

### Content & assets

| Crate / pattern | Role |
|-----------------|------|
| **`tfs-rust-content`** | OTB, OTBM, `items.xml`, vocations, outfits, mounts, spawns, groups |
| **`data/` datapack** | Streamlined TFS scripts/XML (see [Datapack](#datapack) below) |
| **`data/world/`** | OTBM map (`forgotten.otbm` etc.) |

### Scripting

| Crate / pattern | Role |
|-----------------|------|
| **`mlua` + LuaJIT (vendored)** | Script VM in `tfs-rust-lua` |
| **`LuaRuntime` / `ScriptLoader`** | Load `data/lib/`, creaturescripts, actions |
| **`LuaEventDispatcher`** | Replaces `NullEventDispatcher`; `onLogin` / equip hooks wired |
| **`Player` / `Item` userdata** | Inventory-facing Lua API (Track 2 of Lua strategy) |

### Logging & errors

| Crate / pattern | Role |
|-----------------|------|
| **`tracing` + `tracing-subscriber`** | Structured logs; `RUST_LOG` env filter |
| **`anyhow`** | Top-level / boundary errors |
| **`thiserror`** | Domain errors in net/db/common |

### Config

| Source | Role |
|--------|------|
| **`config.lua`** | Gitignored local config (bind addresses, MySQL, ports) |
| **`config.lua.dist`** | Committed defaults |
| **`NetConfig` / `DbConfig`** | Typed views in `crates/tfs-rust-core/src/config.rs` |
| **Env overrides** | `DATABASE_URL`, `TFS_GAME_PORT`, `TFS_PUBLIC_IP`, `TFS_RSA_PEM`, `TFS_CONFIG`, etc. |

### Legacy reference (not executed at runtime)

| Asset | Role |
|-------|------|
| **`src/*.cpp`** | C++ reference for 1:1 parity ports |
| **`key.pem`** | Standard OT protocol RSA key (same as upstream TFS) |

---

## Architecture

```
┌─────────────────────┐         mpsc          ┌──────────────────────┐
│  Tokio I/O tasks    │  ── GameCommand ──►   │  Game thread         │
│  (tfs-rust-net)     │                       │  (tfs-rust-core)     │
│                     │  ◄── output queue ──  │                      │
│  - Accept TCP       │                       │  - GameWorld         │
│  - XTEA / RSA       │                       │  - SlotMap entities  │
│  - Parse packets    │                       │  - Map / walk / items│
│  - SQLx queries     │                       │  - 50 ms tick loop   │
└─────────────────────┘                       └──────────────────────┘
```

**Rule:** I/O never mutates `GameWorld` directly. All simulation changes happen on the game thread.

---

## Crate map

| Crate | Status | Responsibility |
|-------|--------|----------------|
| `tfs-rust-common` | ✅ Complete | Shared types, `Position`, opcodes, `GamePacket`, errors |
| `tfs-rust-content` | ✅ Complete (parity pass ongoing) | OTB/OTBM/XML loaders, item type database |
| `tfs-rust-db` | 🟡 Queries done, wiring partial | Accounts, players, houses, market SQL |
| `tfs-rust-net` | 🟡 Functional | Protocol encode/decode, connection lifecycle |
| `tfs-rust-core` | 🟡 Core sim exists | Game loop, world, walk, items, containers, inventory, combat skeleton |
| `tfs-rust-lua` | 🟡 Partial | VM, script load, Player/Item bindings, event dispatcher |
| `tools/packet-proxy` | Dev tool | Protocol debugging |

---

## Phase roadmap — where we're at

| Phase | Focus | Status |
|-------|-------|--------|
| **A** | Protocol wire-format parity | ✅ Done (A.1 duration byte intentionally skipped for OTCv8) |
| **B** | Item runtime & containers | ✅ Done |
| **C** | Inventory & equipment | ✅ Done |
| **D** | Monster/NPC walk + AI + spawns | ❌ Not started |
| **E** | Combat (melee/distance/magic/death/loot) | 🟡 Skeleton only |
| **F** | Chat & social | ❌ Mostly stubs |
| **G** | Condition ticks (poison, haste, regen) | ❌ Merge rules exist; ticks missing |
| **H** | Spells & runes | ❌ Gating exists; execution missing |
| **I** | NPC / player trade | ❌ Not started |
| **J** | Lua full API | 🟡 Infrastructure + inventory hooks |
| **K** | Persistence & shutdown | 🟡 Save on logout/SIGINT; auto-save/houses open |
| **L** | Outfits, quests, market UI, GM cmds, etc. | ❌ Not started |

Detailed checklists live under `tasks/` (e.g. `tasks/02-phase-A.md` … `tasks/08-roadmap.md`).

---

## Working today

- **Startup:** `./scripts/run_server.sh` or `cargo run --bin tfs-rust` — login port 7171, game port 7172
- **Login sequence:** RSA handshake, character list, self-appear, map description, stats, skills, light
- **Map:** OTBM load, quadtree spectators, known-creature set, floor up/down (0xBE/0xBF)
- **Walking:** Step, diagonal, auto-walk, speed formula, cancel-walk, creature turn packets
- **Items:** Rich `Item` attributes, containers, `internal_move_item`, throw/move, container UI packets
- **Inventory:** Equip/deequip, capacity, quick-equip, real 0x78 inventory packets
- **Save:** Build save blob + `PlayerStore::save_player` on disconnect and SIGINT
- **Lua:** Script load path; `onLogin` and inventory-related hooks end-to-end

---

## Open work (current priorities)

From `tasks/todo.md`:

1. **Throw destination validation** — C++ `Game::playerMoveItem` gating before `internal_move_item`
2. **Container move bug** — don't remove source until destination accepts insert (failure path)
3. **Item parser parity pass** — OTB/XML coverage, nested attributes, container group truth, tests
4. **Phase D** — spawn instantiation, monster/NPC walk extension, basic AI
5. **Phase E** — wire combat formulas and attack cycle on top of existing skeleton

Known container UI issues documented in `tasks/container-bugs.md` (pagination flag, `NeedExchange` on occupied slots).

---

## Datapack

Recent commit **`d519b58`** overhauled `data/`:

- Removed **500+ `.bak` NPC backups**, custom Mod monsters, and sprawling per-city quest script trees
- Added **`data/monster/monsters/*.xml`** (741 per-monster XML files)
- Trimmed dev-only talkactions and test globals
- `.gitignore` now excludes `*.backup`, `data/npc/backup/`, and runtime `data/logs/*`

Game scripts still load from standard TFS layout: `actions/`, `creaturescripts/`, `globalevents/`, `movements/`, `npc/`, `spells/`, `talkactions/`, `weapons/`, `XML/`, `world/`.

---

## Compatibility mandate

- **Default:** exact 1:1 parity with TFS 1.4.2 observable behavior
- **Documented deviations:** e.g. `Cylinder` enum vs C++ vtable; item template duration byte omitted for OTCv8 wire compat
- **Every substantial port** should cite C++ file + function in module comments

---

## Verification commands

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
SQLX_OFFLINE=true cargo test --workspace   # CI-equivalent (no DB)
```

Manual smoke: OTClient 10.98 → `127.0.0.1:7171` / game port from `config.lua`. See `docs/phase1-otclient-checkpoint.md`.

---

## Related docs

| File | Contents |
|------|----------|
| `tasks/08-roadmap.md` | Recommended execution order |
| `tasks/todo.md` | Active task list |
| `tasks/lessons.md` | Rust-specific parity lessons (RSA, login offsets, save) |
| `docs/OTCLIENT_INFO.md` | OTCv8 protocol quirks |
| `docs/WALKING_IMPLEMENTATION_PLAN.md` | Walk system detail |
| `README.md` | Public project overview |

---

## Bottom line

**Solid:** architecture, protocol, login, map, walking, items, containers, inventory, partial Lua, config/save plumbing.  
**Next leap:** stabilize item moves → bring the world alive (monsters/NPCs) → wire combat → layer chat/spells/trade on top of the item foundation.
