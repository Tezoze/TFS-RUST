# Talkactions Lua — phased implementation plan

**Policy:** [docs/DATA_PACK_LUA.md](../docs/DATA_PACK_LUA.md). Companion: [data-pack-lua-implementation-plan.md](data-pack-lua-implementation-plan.md).

Talkactions already **load and dispatch** (`TalkAction:register` + `talkactions.rs`). Failures are missing **pack primitives**, not the registry.

**Live log (2026-08-24):** `db` nil (`/deathlist`), `setGhostMode` nil (`/ghost`), `getIp` nil (`/info`, `/mccheck`). `/kick` against another access player is **correct** (`group:getAccess()` → “You cannot kick this player”).

`Game.convertIpToString` already exists in [data/lib/core/game.lua](../data/lib/core/game.lua). Binding `player:getIp()` is enough for `/info` and `/mccheck`.

---

## Policy conflict — `db` / `result`

[data-pack-lua-implementation-plan.md](data-pack-lua-implementation-plan.md) Phase 3.3 chose **(b) no `db`/`result`** for slim `playerdeath.lua`. `/deathlist`, `/ipban`, `/ban`, `/unban`, `/removetutor` **cannot** run without SQL.

**This plan reverses that for the Lua surface:** bind a narrow TFS-shaped `db` / `result` over the existing sqlx pool. Death **mechanics** stay native; Lua SQL is pack policy (GM tools + optional death rows). Lesson 369 (“no pack `db` global”) is superseded for this surface only.

Threading: queries run on the **IO/sqlx** side; the game thread **waits on a oneshot**. Never `tokio::spawn` work that touches `GameWorld`.

---

## Architecture (no hub growth)

| Concern | Where |
|---------|--------|
| `db` / `result` | New `crates/tfs-rust-lua/src/lua_database.rs` — not `runtime.rs` dump |
| House methods | New `crates/tfs-rust-lua/src/userdata/house.rs`; `tile.rs` `HouseRef` currently only has `getId` |
| Mutations | Extend `lua_mutation.rs` + `ScriptContext` / thin `GameWorld` delegates (`lua_scope.rs`) |
| IP | Session IPv4 `u32` on live `Player` at login (`players.lastip` already in DB) |
| Ghost write | Native `ghost_mode` already used for visibility; **read** bound (`isInGhostMode`); **write** missing |

C++ names in `//!` comments (`luaPlayerGetIp`, `luaPlayerSetGhostMode`, `luaDatabaseStoreQuery`, `luaResultGetNumber`, `luaPlayerRemove`, `luaHouseSetOwnerGuid`, …). Repo `src/` has no `luascript.cpp`.

772 has no TFS FYI box opcode. `player:popupFYI` maps to the existing `0x96` text dialog (`showTextDialog`) with a dummy item id (1950 letter) — document in lessons.

---

## Inventory (55 scripts under `data/scripts/talkactions/`)

Already bound (do not re-bind): `getGroup`/`getAccess`, `getAccountType`, `getName`, `getLevel`, `getMagicLevel`, `getPosition`, `teleportTo`, `sendTextMessage`, `sendCancelMessage`, `isInGhostMode`, `Game.getPlayers`, `Game.saveServer`, `Game.createMonster`, `Game.createItem`, `Game.createTile`, `Town`/`getTemplePosition`, `Tile` basics, `addItem`, `addHealth`, `addSkillTries`, `addCondition`/`removeCondition`, `setOutfit`/`getOutfit`, `getSex`, `isPremium`, `getGuid`, `registerEvent`.

**Missing — Phase 1:** `getIp`, `setGhostMode`, `popupFYI`.

**Missing — Phase 2:** `player:remove` / `creature:remove` (kick/logout, not `item:remove`).

**Missing — Phase 3:** `db.storeQuery` / `db.query` / `db.asyncQuery` / `db.escapeString`; `result.getNumber` / `getString` / `next` / `free`.

**Missing — Phase 4:** `Game.getHouses`; House `getName`, `getExitPosition`, `getOwnerGuid`, `setOwnerGuid`, `isGuildHall`, `getRent`, `getTileCount`, `getTown`, `startTrade`; `tile:getHouse` already returns `HouseRef`.

**Missing — Phase 5:** `Game.createNpc`, `npc:setMasterPos`, `Game.setGameState`, `Game.startRaid` / `getReturnMessage`, `Game.unlockAccount` / `unlockIp`, `getIPByName`/`getIPNumberFromString` as used by unlock scripts, `refreshMap` (no-op+log if map reload out of scope), `Game.reload` slim, tile `getTopVisibleThing` / thing attributes / `item:decay` as needed.

**Missing — Phase 6:** `removeTotalMoney`, premium days get/set, `setSex`, party share, skull/murder (partially bound as `getMurderTimestamps`), `canSeeCreature`, `getWorldUpTime`, `configManager.getNumber` real keys.

---

## Phase 0 — Document — **this file**

Exit: this plan exists; `tasks/todo.md` tracks execution.

---

## Phase 1 — Logged GM (no SQL)

**Unblocks:** `/ghost`, `/info`, `/mccheck`.

1. Add `lastip: u32` on `Player` (`creature/player.rs`). Set at login from the TCP peer (`PlayerLogin` already has `peer` in net). Persist path already writes `players.lastip`.
2. `ScriptContext::get_player_ip` + `player:getIp()` — TFS `luaPlayerGetIp` returns `uint32`.
3. `LuaMutation::PlayerSetGhostMode` → flip `ghost_mode` + spectator/map refresh (same visibility paths as login_out / walk). `player:setGhostMode(bool)`.
4. `player:popupFYI(text)` → `showTextDialog(1950, text)` (772 has no FYI packet).

Tests: IP endianness matches `Game.convertIpToString`; ghost write readable via `isInGhostMode`.

---

## Phase 2 — Kick / remove

**Unblocks:** `/kick` on non-access targets; `/ban` / `/ipban` after Phase 3.

`LuaMutation::CreatureRemove` → `player_logout(..., forced=true)` for players; `remove_creature` for monsters/NPCs. Bind on `CreatureRef` as `remove`.

Do **not** confuse with `item:remove`.

---

## Phase 3 — `db` / `result`

New `lua_database.rs`. Register `db` table + `result` module in `runtime.rs` (thin call).

Bridge: `register_lua_db_bridge` (same OnceLock pattern as mutation applier). Core implements with `DbPool`: spawn blocking query on runtime, oneshot wait on game thread.

- `db.escapeString(s)` — local, no IO.
- `db.query(sql)` — execute, return bool.
- `db.storeQuery(sql)` — SELECT; return result id or `false`.
- `db.asyncQuery(sql)` — same as `query` for now (TFS queues; we block like dispatcher MySQL).
- `result.getNumber(id, col)` / `getString` / `next` / `free`.

Result store is game-thread-only (`RefCell<HashMap>`). Unit tests with a mock bridge (no live MySQL required).

Unblocks: `/deathlist`, `/ipban`, `/ban`, `/unban`, `/removetutor`.

---

## Phase 4 — House

Move `HouseRef` methods into `userdata/house.rs`. Extend `HouseManager` (or a sibling `HouseCatalog`) with name, exit, rent, size, town, guild-hall from `data/world/*-houses.xml` if not already loaded.

`Game.getHouses()` → array of `HouseRef`. `setOwnerGuid` → existing `HouseManager::set_owner`. `startTrade` may return false until house trade is native.

Unblocks: `/gotohouse`, `/owner`; buy/leave/sell still need Phase 6 money.

---

## Phase 5 — God spawn/admin

- `Game.createNpc(name, pos)` → existing `GameWorld::spawn_npc`.
- `npc:setMasterPos(pos)` — store on NPC (home/master pos already on spawn).
- `Game.setGameState(state)` — existing `GameState` enum (lesson 373 said not to bind; this plan **does** bind for `/closeserver` `/openserver`).
- `Game.startRaid` / `Game.getReturnMessage` — wire if raid runner exists; else return fail message.
- `Game.unlockAccount` / `unlockIp` — DB or in-memory ban list.
- `getIPNumberFromString` global used by `/unlockip`.
- `Game.reload` slim: rescan talkactions/scripts (`scripts_interface` already re-runnable); other types log + true.
- `refreshMap` — no-op + log if full remap is out of scope.

---

## Phase 6 — Players / tutors leftovers

Bind only what remaining scripts call: `removeTotalMoney`, premium days, `setSex`, party shared XP, `getSkull`, `canSeeCreature`, `getWorldUpTime`, `configManager.getNumber` for `RATE_*`, `HOUSE_PRICE`, skull/banish kill counts.

---

## Phase 7 — Close-out

- `rtk cargo run -p tfs-rust-lua --bin emit-lua-defs`
- Tests per crate
- `tasks/lessons.md`: db reversal; popupFYI → 0x96; setGameState bound despite globalevents lesson

---

## Out of scope

Rewriting talkaction Lua; full `Game::reload` clone; stuffing new subsystems into `game_world.rs`.

## Verify (each phase)

```
rtk cargo test -p tfs-rust-lua --lib
rtk cargo test -p tfs-rust-core --lib -- ghost_mode get_ip
rtk cargo run -p tfs-rust-lua --bin emit-lua-defs
```

Live: `/ghost`, `/info <name>`, `/mccheck`, `/deathlist <name>`, `/kick` on a **player** character.
