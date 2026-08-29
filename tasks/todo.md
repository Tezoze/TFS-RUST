# 772 client crash: unknown packet type 29 (0x1D)

**Status:** complete.
**Symptom:** Official 7.72 `Control.cpp:1274` — `unknown packet type during game (Type = 29)`. Last packet `001 000 029` (1-byte `0x1D`). Last types `029` then many `109` (`0x6D` moves). Feels random because keepalive only fires after ~30/60 idle rounds (or lag `NetLoadCheck`).

## Cause

`server::SEND_PING = 0x1D` is **1098 / OTClient**. Official 772 keepalive is **`0x1E`** (`ProtocolGame::sendPing` in TVP `protocolgame.cpp:1516-1524`).

`tick_player_pings` already used `codec.periodic_ping_packet(is_otclient)`. The live 772 path did not:

- `connections.rs` `process_connections` — always `send_ping()` (`0x1D`) at LastCommand 30/60
- `game_world_tick.rs` `net_load_check` — same under lag

## Fix

Shared `GameWorld::enqueue_periodic_ping`. Tests: official 772 → `0x1E`; OTClient → `0x1D`; 1098 always `0x1D`.
