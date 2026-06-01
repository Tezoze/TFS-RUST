# 772 OTClient / OTCv8 parity — audit & work plan

**Status:** backlog (not started)  
**Authority for 772 wire:** `gameserver/src/` only — see `.cursor/rules/TFS-protocol-versioning.mdc`  
**Companion:** `docs/PROTOCOL_VERSIONING.md`, `docs/PROTOCOL_VERSIONING_IMPLEMENTATION_PLAN.md`  
**1098 OTC notes (different era):** `docs/OTCLIENT_INFO.md`

This document captures a parity audit (Rust `clientVersion = 772` vs TVP `gameserver/`) and rough effort estimates. Use it to schedule work later; it is not a binding phase gate.

---

## Executive summary

| Question | Answer |
|----------|--------|
| Does 772 C++ advertise `GameNewWalking` / `sendFeatures` (`0x43`)? | **No** — not in `gameserver/`. |
| Does Rust 772 block those vs C++? | **No** — omitting them on the 772 login path is **correct** for TVP parity. |
| Are there real gaps? | **Yes** — wire quirks, gameplay branches, and Lua extended opcode are incomplete. |
| Rough effort | **~1 day** wire + gameplay branches; **+1–2 days** if Lua `onExtendedOpcode` must work. |

---

## What `gameserver/` actually implements

C++ references (grep anchors):

- `gameserver/src/protocolgame.cpp` — login preamble, first packet, `parsePacket`, `parseUseItem`, `parseThrow`, `parseSay`, `sendPing`, `sendAddTileItem`, `sendAddCreature`, `sendOutfitWindow`, `parseExtendedOpcode`
- `gameserver/src/protocolgame.h` — `otclientV8` member
- `gameserver/src/game.cpp` — `parsePlayerExtendedOpcode`
- `gameserver/src/creatureevent.cpp` — `executeExtendedOpcode`

### Supported (772 TVP)

| Area | Condition | Behavior |
|------|-----------|----------|
| Login extended-opcode init | `otclientV8 \|\| operatingSystem >= CLIENTOS_OTCLIENT_LINUX` | Outgoing `0x32 0x00` + `u16` length `0` (empty buffer) |
| First game packet preamble | `operatingSystem >= CLIENTOS_OTCLIENT_LINUX` only | Same `0x32` init **before** credentials are read (`onRecvFirstMessage` ~349–355) |
| OTCv8 probe | After credentials | `u16` len `5` + `"OTCv8"` + `u16` build → sets `otclientV8` |
| Creature event | `operatingSystem >= CLIENTOS_OTCLIENT_LINUX` on login | `registerCreatureEvent("ExtendedOpcode")` |
| Incoming `0x32` | Always in `parsePacket` | `parseExtendedOpcode` → `Game::parsePlayerExtendedOpcode` → Lua `onExtendedOpcode` |
| Instant use | `otclientV8` | `playerUseItem` directly (no `toDo` / wait queue) |
| Instant throw | `otclientV8` | `playerMoveThing` directly (no 100 ms scheduler / `toDo`) |
| Direct say | `otclientV8` | `playerSay` directly (stock client cancels walk + queues say) |
| `0x6A` stackpos | `operatingSystem >= CLIENTOS_OTCLIENT_LINUX` | Extra `u8` stackpos on `sendAddTileItem` / non-self `sendAddCreature` / some teleport re-add paths — **implemented** in Rust (`codec/v772.rs`, `game_world.rs`, `spawn_lifecycle.rs`) |
| Server-initiated ping | `operatingSystem >= CLIENTOS_OTCLIENT_LINUX` | `sendPing` uses opcode **`0x1D`** (not `0x1E`) |
| Outfit window | `operatingSystem >= CLIENTOS_OTCLIENT_LINUX` | `0xC8` uses OTClient layout (current outfit + name list) |

### Not present in `gameserver/` (do not add for “772 parity”)

| Feature | Notes |
|---------|--------|
| `sendFeatures` / opcode `0x43` | Repo-root TFS 1.4.2 `src/` has this for **1098**; TVP 772 does not. |
| `GameNewWalking` | No `parseNewWalking`; client must not rely on server feature `90` on 772. |
| Item `withDescription` on wire | 772 `NetworkMessage::addItem` has no description branch — template items are minimal (`id` + count/liquid). |

---

## Rust 772 — parity checklist

| Feature | `gameserver/` | Rust today | Verdict |
|---------|---------------|------------|---------|
| Parse `"OTCv8"` probe | Yes | Yes — `crates/tfs-rust-net/src/game_first_packet.rs` | OK |
| Login burst `0x32` init | Yes | Yes — `crates/tfs-rust-core/src/login_out.rs` (`enqueue_initial_login_packets_772`) | OK |
| First-packet `0x32` (pre-auth) | Yes | No — `crates/tfs-rust-net/src/server.rs` | **GAP** |
| Incoming `0x32` accepted | Yes | Yes — `protocol_opcodes`, `game_parse`, `game_loop` | OK |
| Lua `onExtendedOpcode` | Yes | `NullProtocolHooks` — no-op | **GAP** |
| Register `ExtendedOpcode` event on login | Yes | Not wired (`CreatureEventType` has no ExtendedOpcode in loader) | **GAP** |
| `sendFeatures` / `GameNewWalking` | No | Not on 772 path (1098 burst still sends features — separate issue) | OK (N/A) |
| `otclientV8` instant use | Yes | `player_use_item` always defers on `nextAction` | **GAP** |
| `otclientV8` instant throw | Yes | Always immediate; no stock delayed path | **GAP** (partial) |
| `otclientV8` direct say | Yes | Say not action-gated at packet level | Likely OK |
| `0x6A` + stackpos for OTClient | Yes | `Codec772` + per-conn `conn_uses_772_otclient_stackpos` | **OK** |
| Server ping `0x1D` for OTClient | Yes | `send_ping()` always `0x1E` — `player_ping.rs` | **GAP** |
| Item description bytes on 772 wire | No | `Codec772::write_item_template` ignores `with_description` | OK |
| Outfit window `0xC8` OTClient | Yes | Not wired in core (1098 helper in `outgoing_extra.rs` only) | Defer |

### Code anchors (Rust)

```text
# Login OTC preamble (matches gameserver login())
crates/tfs-rust-core/src/login_out.rs  ~431–435

# 772 codec — OTClient stackpos intentionally omitted today
crates/tfs-rust-net/src/codec/v772.rs  ~12–15, encode_add_tile_item / encode_add_tile_creature

# Extended opcode stub
crates/tfs-rust-core/src/protocol_hooks.rs  NullProtocolHooks

# Player OTC flags
crates/tfs-rust-core/src/creature/player.rs  otclient_v8, item_with_description(), operating_system
```

### Lua / data

- Script exists: `data/creaturescripts/scripts/extendedopcode.lua`
- Loader today: `crates/tfs-rust-lua/src/script_loader.rs` — `CreatureEventType` = `Login` \| `Logout` \| `Death` only
- Runtime: `crates/tfs-rust-core/src/lua_event_dispatcher.rs` — no extended-opcode dispatch

---

## Effort estimates

Rough focused-dev time (single engineer, familiar with codebase).

| Gap | Effort | Priority / impact |
|-----|--------|-------------------|
| First-packet `0x32` | 1–2 h | High — handshake before auth |
| Server ping `0x1D` vs `0x1E` for OTClient | 1–2 h | Medium — keepalive opcode mismatch |
| `0x6A` stackpos on 772 for OTClient | 4–8 h | High — visible map/creature add bugs on OTC |
| `otclientV8` instant `useItem` | 2–3 h | Medium — action feel |
| `otclientV8` instant `throw` + stock delayed path | 3–6 h | Medium — parity is two code paths |
| Lua `onExtendedOpcode` + per-player registration | 1–2 days | High if custom OTC modules / scripts required |
| Outfit window `0xC8` OTClient layout | 0.5–1 day | Low — defer until outfit UI needed on 772 |
| Verification (golden bytes + OTClient smoke) | 2–4 h | Always |

**Totals**

- **Wire + gameplay branches only (no Lua):** ~1 day (4–8 h focused + tests)
- **Full gameserver OTC parity including scripts:** ~2–4 days
- **Out of scope for 772 TVP parity:** `GameNewWalking`, `sendFeatures` (`0x43`) — OTClient extension beyond `gameserver/`

---

## Suggested implementation order

1. **First-packet `0x32`** — `server.rs`: after `parse_first_client_packet`, if game packet and `operating_system >= CLIENTOS_OTCLIENT_LINUX`, queue `send_extended_opcode(0, "")` before DB auth.
2. **OTClient server ping** — `player_ping.rs` + outgoing: branch on `Player::operating_system` (mirror `ProtocolGame::sendPing`).
3. **`0x6A` stackpos** — extend `Codec772` (or codec trait) with OTClient flag; pass `player.item_with_description()` or `operating_system >= CLIENTOS_OTCLIENT_LINUX` from `GameWorld` encode call sites; add golden tests for OTClient variant.
4. **`otclientV8` use / throw** — branch in `container_ui::player_use_item` and `game_world::player_move_thing` (add delayed path for non-OTC if emulating stock 772).
5. **Lua extended opcode** — add `CreatureEventType::ExtendedOpcode`, per-player `registered_events`, real `ProtocolHooks` impl, register on login, wire `game_loop` → Lua.

---

## Verification

```bash
# C++ reference scan
rg -n "otclientV8|CLIENTOS_OTCLIENT|parseExtendedOpcode|sendFeatures" gameserver/src/protocolgame.cpp

# Rust net tests
cargo test -p tfs-rust-net protocol_compat -- --nocapture
cargo test -p tfs-rust-net map_description -- --nocapture
```

**Manual (772 + OTClient):**

- Connect with `CLIENTOS_OTCLIENT_LINUX` and optional `"OTCv8"` probe.
- Confirm first-packet and login `0x32` preamble.
- Walk, throw item, use item — compare latency vs stock client.
- Trigger `extendedopcode.lua` (only after Lua wiring).

---

## Related plan items

- **1098 login** still sends `send_otcv8_features` unconditionally — plan A6.2 / `tasks/lessons.md` (OTClient-gated on 1098, not this doc’s 772 scope).
- **Track A wire** marked done in `PROTOCOL_VERSIONING_IMPLEMENTATION_PLAN.md`; this doc is an **OTClient-on-772 overlay** on top of A5 772 codec work.

---

## Changelog

| Date | Note |
|------|------|
| 2026-06-01 | Initial audit from gameserver vs Rust comparison (login perf / OTCv8 thread). |
