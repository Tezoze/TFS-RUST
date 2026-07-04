# 772 Floor Change — Two Client Targets (OTClient vs Real 7.72 Client)

**Status:** Design note / decision record
**Scope:** Player self floor changes (stairs, ladders, ropes, holes) on `clientVersion = 772`
**Related:** `docs/772_FLOOR_CHANGE_DESYNC.md`, `docs/772_OTCLIENT_PARITY.md`, `.kiro/steering/tfs-wire-codec.md`

---

## 1. The core problem

A 772 server can be talked to by **two different clients**, and they do **not** agree on how a
player's own floor change should be encoded on the wire:

| Client | Renders local player as… | Reference server (ground truth) |
|--------|--------------------------|---------------------------------|
| **Real Tibia 7.72 client** | Screen-centered self (NOT a tile creature) | CipSoft decompile — `reference/cipsoft-772/tibia-game-master/src/` |
| **OTClient / OTCv8** | A `Creature` placed on a tile (like 1098) | TVP — `reference/tvp-772/gameserver/src/` |

Because the two clients model "where am I" differently, one byte stream does **not** obviously
satisfy both. The map **body** (floor descriptions + edge rows) is identical for both; the
divergence is entirely in **how the player's own move is announced** (the leading self-packet
and whether a floor change is treated as an incremental update or a full-screen teleport).

> **Steering note.** `.kiro/steering/tfs-wire-codec.md` designates `gameserver/src/` (TVP) as the
> 772 wire reference. That is correct when the target is OTClient. When the target is the **real
> 7.72 client**, the CipSoft decompile is the ground truth (it is literally the server that client
> was built against). This document records both so the choice is explicit, not accidental.

---

## 2. Confirmed: the map body format is already correct for both

The floor-change-down body the Rust server emits is byte-identical to TVP's `GetTileDescription`
and to the decompile's `SendMapPoint`. Verified byte-level:

```
0xBF                                  -- floor-down opcode (1 byte, NO position prefix)
[floor descriptions for z=8,9,10]     -- skip-compressed tiles
  each tile: u16 groundClientId [u8 count]   -- NO groundSpeed, NO blocking byte
             | top items | creatures | down items
0x66                                  -- east row opcode
[1 column, full height]               -- skip-compressed
0x67                                  -- south row opcode
[full width, 1 row]                   -- skip-compressed
```

- **No** `(x,y,z)` prefix on `0xBF` — `map_description.rs` `append_move_down_creature`.
- **No** `groundSpeed(u16)` / `blocking(u8)` per tile — `codec/v772.rs` tile writer; matches TVP
  `GetTileDescription` (`protocolgame.cpp` ~539-548), whose `addItem(ground)` is just
  `u16 clientId` + optional count.
- **No** environment-effect `u16` — 772 `write_tile_environment_prefix` is a no-op (`mod.rs`).

So the OTClient "position prefix / groundSpeed / environment" concerns do **not** apply to this
server. The body is not the problem. The divergence is the **self-move announcement** that precedes
the body (and, separately, whether the map state left the creature on the old tile — see §5).

---

## 3. Part 1 — Match OTClient (TVP behavior)

OTClient tracks the **local player as a tile creature**, so it needs an explicit signal that the
player left the old tile and arrived on the new one. This is what TVP (`gameserver/`) sends.

### TVP `Map::moveCreature` (`map.cpp:285`)

```cpp
bool teleport = forceTeleport || !newTile.getGround()
              || !Position::areInRange<1, 1, 0>(oldPos, newPos);
```

`areInRange<1,1,0>` requires `dz == 0` (`position.h:33`), so **any z-change ⇒ teleport = true**.

### TVP `ProtocolGame::sendMoveCreature` self branch (`protocolgame.cpp:1766`)

- **Teleport (every floor change):**
  - `sendRemoveTileCreature(oldPos)` — skipped only for `newPos.z == 8 && oldPos.z == 7`.
  - `sendMapDescription(newPos)` — a full-screen `0x64`.
- **Same-z step (non-teleport):**
  - `0x6D` self-move (`oldPos` + stackpos, or `0xFFFF` + creatureId) + `newPos`, then edge rows.

**Net for OTClient:** self floor changes are **full-screen `0x64` redraws**; same-z steps use `0x6D`.
The local-player tile creature is relocated by the remove + re-add inside the `0x64`, or by the
`0x6D` on flat ground.

### Rust status for this target

- Self-packet: `move_creature_self_packet` cap (`protocol_version.rs`) — set **true** to emit it.
- Teleport classification: `is_adjacent_move` must use `are_in_range_1_1_0` (dz == 0) so z-changes
  route through `emit_teleport_move_packet` (which already does remove + `0x64`).

---

## 4. Part 2 — Match the decompile (real 7.72 client)

The real 7.72 client draws the local player at the **screen center from its own position** and does
**not** track the self as a tile creature. It never expects a self-move packet; it repositions the
viewport purely from the floor/row stream.

### Decompile `TCreature::NotifyGo` (`cract.cc:1400-1465`)

```cpp
if (DistanceX <= 1 && DistanceY <= 1 && DistanceZ <= 1) {
    while (posz < DestZ) { posx--; posy--; posz++; SendFloors(Connection, false); } // 0xBF
    while (posz > DestZ) { posx++; posy++; posz--; SendFloors(Connection, true);  } // 0xBE
    while (posx < DestX) { posx++; SendRow(Connection, EAST);  }  // 0x66
    while (posx > DestX) { posx--; SendRow(Connection, WEST);  }  // 0x68
    while (posy < DestY) { posy++; SendRow(Connection, SOUTH); }  // 0x67
    while (posy > DestY) { posy--; SendRow(Connection, NORTH); }  // 0x65
} else {
    posx = DestX; posy = DestY; posz = DestZ;
    SendFullScreen(Connection);   // 0x64
}
```

**Key properties:**
- **No `0x6D`/`0x6C` for the player's own move — ever.**
- Adjacent (`dx,dy,dz ≤ 1`): incremental `SendFloors` (0xBE/0xBF) + `SendRow` (0x65-0x68),
  updating `posx/posy/posz` step-by-step so each send reads the current position.
- Non-adjacent: `SendFullScreen` (0x64).

### Rust status for this target

- Self-packet: `move_creature_self_packet` cap set **false** (suppress the leading `0x6D`/`0x6C`).
  This is the committed Phase 1 behavior; the current working tree re-added the self-packet, which
  reintroduces the desync on the real client (`bug000017` + double-shift).
- Teleport classification: `is_adjacent_move` uses `are_in_range_1_1_1` (dz ≤ 1) so adjacent floor
  changes take the incremental `SendFloors`/`SendRow` path (`append_move_down_creature` /
  `append_move_up_creature`), matching `NotifyGo`. Only `dz > 1` (or dx/dy > 1) → `0x64`.

---

## 5. Open item — server-side tile state ("creature left on old tile")

Independent of the self-packet, verify the **map state** actually relocates the creature on a floor
change. `move_creature_on_map` (`walk/mod.rs`) calls `unregister_creature_at(old)` +
`register_creature_at(new)`, so single-segment floor changes should be clean. Two things to check
before concluding:

1. **Chained floor changes** (queryDestination up/down after landing): the Rust step runs *all*
   `move_creature_on_map` calls first, then emits packets, and broadcasts spectators **once** with
   the overall `old → new` — TVP emits **per segment**. An overall `old → new` that is non-adjacent
   can produce a cross-floor spectator `0x6D` that leaves the creature on the old tile for observers.
2. **Spectator branch** (`broadcast_spectator_move`): currently remove+add only for
   `surface_to_underground`; other z-changes send `0x6D`. Confirm this matches the chosen client
   target's expectation.

---

## 6. Recommended resolution — gate per connection

The self-packet is already capability-gated, and the codebase already branches per-connection on
OTClient (`conn_uses_772_otclient_stackpos`, `codec/v772.rs`, driven by the
`CLIENTOS_OTCLIENT_LINUX` / `"OTCv8"` probe). Use the same seam here:

- **Real 7.72 client connection:** decompile-faithful — no self-packet, incremental
  `SendFloors`/`SendRow` (Part 2).
- **OTClient connection:** TVP-faithful — self-packet + `0x64` teleport for z-changes (Part 1).

This keeps the shared 772 map body identical (it already is) and treats the self-move announcement
as a per-client quirk, which is how the steering says to handle OTClient-specific deviations.

**§6 experiment result (2026-07-04): FAILED.** Suppressed the self-packet for 772 (Part 2,
decompile-faithful) and tested with both OTClient and the real 772 client. **Both desynced
immediately — the player could not move at all.** The self-packet (`0x6D`/`0x6C`) is REQUIRED
for both clients to update their central position before the floor/row stream is processed.
The decompile's `NotifyGo` model (no self-packet, viewport shifts purely via `SendFloors` +
`SendRow`) does **not** work with our server's packet flow. The `bug000017` log is a debug
warning, not a desync.

**§6 per-connection fix (2026-07-04): IMPLEMENTED.** The §6 experiment only tested suppressing
the self-packet while keeping the incremental `SendFloors`/`SendRow` body — it never tested
routing floor changes through TVP's **teleport** path (`emit_teleport_move_packet`: remove +
`0x64` full screen), which is what OTClient is actually built for. The real bug: walking
perpendicular onto a stair (e.g. west onto south-facing stairs) leaves a leftover delta on
both axes after the diagonal z-shift, producing a row sequence OTClient can't reconcile after
the `0x6D` pre-jumps the self to the final tile. The parallel approach (north onto south
stairs) shifts along the stair's own axis, so the row sequence stays consistent enough that
OTClient doesn't throw.

**Fix:** the walk dispatch (`walk/mod.rs:~1750`) now branches on the connection's OTClient flag
(`Player::is_otclient`, same seam as `0x6A` stackpos / ping opcode). OTClient-on-772 floor
changes route through the per-segment TVP teleport path (`emit_teleport_move_packet`: remove +
`0x64`), while the real 7.72 client keeps the decompile `NotifyGo` incremental path
(`emit_notify_go`: `0x6D` + `SendFloors`/`SendRow`). `is_adjacent_move` now takes an `otclient:
bool` parameter: OTClient-on-772 uses `areInRange<1,1,0>` (dz==0, z-changes are teleports),
matching TVP `protocolgame.cpp:1766-1829`; the real 772 client uses `areInRange<1,1,1>`
(dz≤1, adjacent z-changes are incremental), matching decompile `NotifyGo`.

**Conclusion:** A per-connection fork IS needed. The self-packet is required for both clients,
but the **floor-change body** must differ: OTClient needs TVP's full-screen `0x64` teleport;
the real 772 client needs the decompile's incremental `SendFloors`/`SendRow`. The §6
experiment's "no per-connection fork" conclusion was wrong — it only tested the self-packet
suppression, not the teleport-vs-incremental body dispatch.

---

## 7. Verification

```bash
cargo test -p tfs-rust-net --test protocol_compat
cargo test -p tfs-rust-net --test map_description
cargo test -p tfs-rust-core walk
```

Manual smoke test per target client: hole down, ladder up, stairs (diagonal), rope up, teleport
scroll — confirm no viewport/creature desync on each.

---

## 8. References

| What | Real 7.72 (decompile) | OTClient (TVP) | Rust |
|------|-----------------------|----------------|------|
| Self floor change | `cract.cc:1400-1465` `NotifyGo` | `protocolgame.cpp:1766` `sendMoveCreature` (teleport) | `map_description.rs` `send_move_creature_player`; `walk/mod.rs` `emit_teleport_move_packet` |
| Teleport classification | `NotifyGo` `Distance ≤ 1` | `map.cpp:285` `areInRange<1,1,0>` | `walk/mod.rs` `is_adjacent_move` |
| Floor body | `sending.cc` `SendFloors`/`SendRow` | `protocolgame.cpp` `MoveDownCreature`/`GetTileDescription` | `map_description.rs` `append_move_down_creature` / `append_move_up_creature`; `codec/v772.rs` |
| Self-packet cap | — | — | `protocol_version.rs` `move_creature_self_packet` |
| OTClient per-conn seam | — | `CLIENTOS_OTCLIENT_LINUX` / `OTCv8` probe | `codec/v772.rs` `conn_uses_772_otclient_stackpos` |
