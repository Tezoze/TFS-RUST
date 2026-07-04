# 772 Floor Change Desync — Protocol Divergence Audit

**Status:** Fixed (Phases 1–4 implemented 2026-07-03; era-aware teleport detection 2026-07-04; TVP-aligned self-packet + teleport remove 2026-07-04) — root cause identified and narrowed (audit #2, 2026-07-03); Phase 0 golden tests implemented; era-aware teleport detection routes 772 adjacent z-changes through `SendFloors`/`SendRow`; self-packet and teleport-remove now match TVP exactly (both eras).
**Severity:** High — every player floor transition (stairs, rope, ladder) desyncs the 772 client.
**Reported:** 2026-07-03. **Re-audited:** 2026-07-03 (§16). **Phase 0:** 2026-07-03. **Phases 1–4:** 2026-07-03. **Teleport detection fix:** 2026-07-04. **TVP alignment:** 2026-07-04.

---

## 1. Summary

The Rust server uses the **1098 `sendMoveCreature` packet structure** for the player's own
move on **both eras**. The 772 decompile uses a **different packet structure** (`NotifyGo`)
that does NOT send a self-move creature packet (`0x6D`/`0x6C`) for the player's own move.

> **Audit #2 correction (see §16).** The original worked examples in §9–§10 claimed the Rust
> path also produces a *wrong floor origin* and a *missing map row*. A byte-level re-trace against
> the actual Rust code refutes that: `append_move_up_creature` / `append_move_down_creature`
> emit the edge rows themselves (`0x66/0x67` down, `0x68/0x65` up), and once
> `get_floor_description`'s `offset` (`origin_z - point_z`) is applied, the floor-description
> **tiles are byte-identical to 772's `SendFloors`/`SendRow`**. The map payload matches.
> The **only** guaranteed divergence on the common floor changes is the **spurious leading
> self-creature packet** (`0x6D`, or `0x6C` for surface→underground) that 772's `NotifyGo`
> never emits. A secondary divergence is **row ordering** on combined diagonal+z moves (§16.3).

**Double-shift mechanism (refined).** The 772 client *does* process `0x6D` (it receives it for
spectators). When the player's own move arrives as `0x6D` with the player's own creature ID, the
client moves its own sprite/camera to the new position — but `NotifyGo`'s `SendFloors`/`SendRow`
stream has *already* repositioned the viewport. The result is a **double application** of the
move: once from the (spurious) `0x6D`, once from the floor/row stream. Same-z moves are less
visibly broken because the shift is smaller and self-correcting; z-changes compound the error
across floors.

---

## 2. 772 decompile — `TCreature::NotifyGo`

**Source:** `reference/cipsoft-772/tibia-game-master/src/cract.cc:1400-1460`

`NotifyGo` is the player's own move notification, called from `operate.cc:1431` after
`MoveObject` relocates the creature. It is the 772 analog of 1098's
`ProtocolGame::sendMoveCreature(creature == player)`.

### 2.1 Adjacent move (`DistanceX ≤ 1 && DistanceY ≤ 1 && DistanceZ ≤ 1`)

```cpp
// z-steps FIRST — posx/posy shift diagonally with each z-step
while(this->posz < DestZ){
    this->posx -= 1;
    this->posy -= 1;
    this->posz += 1;
    SendFloors(this->Connection, false);   // 0xBF
}
while(this->posz > DestZ){
    this->posx += 1;
    this->posy += 1;
    this->posz -= 1;
    SendFloors(this->Connection, true);    // 0xBE
}
// then x-steps
while(this->posx < DestX){ this->posx += 1; SendRow(this->Connection, DIRECTION_EAST); }
while(this->posx > DestX){ this->posx -= 1; SendRow(this->Connection, DIRECTION_WEST); }
// then y-steps
while(this->posy < DestY){ this->posy += 1; SendRow(this->Connection, DIRECTION_SOUTH); }
while(this->posy > DestY){ this->posy -= 1; SendRow(this->Connection, DIRECTION_NORTH); }
```

**Key properties:**
- **NO `0x6D` (SendMoveCreature) is sent for the player's own move.** The viewport is updated
  purely via `SendFloors` (0xBE/0xBF) + `SendRow` (0x65-0x68).
- The z-step loop adjusts `posx/posy` by ∓1 per floor change. This is the "diagonal shift" of
  stairs — going down a floor also shifts the player diagonally northwest in world coords.
- The intermediate `posx/posy` (after z-step adjustment, before x/y-step adjustment) is the
  origin used by `SendFloors` for the floor description.
- The remaining x/y delta (after z-step adjustment) generates additional `SendRow` packets.
- The creature's `posx/posy/posz` fields are updated **step-by-step** as the loops progress,
  so `SendFloors` and `SendRow` always read the current position via `Connection->GetPosition`.

### 2.2 Teleport move (`DistanceX > 1 || DistanceY > 1 || DistanceZ > 1`)

```cpp
this->posx = DestX;
this->posy = DestY;
this->posz = DestZ;
SendFullScreen(this->Connection);   // 0x64 (SV_CMD_FULLSCREEN = 100)
```

Full screen map description — no `0x6D`, no `0xBE`/`0xBF`, no `SendRow`.

### 2.3 Post-move side effects (`cract.cc:1462-1560`)

After the packet emission, `NotifyGo` also:
- Closes open containers that are no longer accessible (`ObjectAccessible(self, Con, 1)`)
- Cancels trades if the partner is too far away (`ObjectDistance > 2`)
- Computes `EarliestWalkTime` from the destination tile's `BANK`/`WAYPOINTS` + creature speed
  (diagonal moves cost 3× the waypoints)

---

## 3. 772 `SendFloors`

**Source:** `reference/cipsoft-772/tibia-game-master/src/sending.cc:517-573`

```cpp
void SendFloors(TConnection *Connection, bool Up){
    int PlayerX, PlayerY, PlayerZ;
    Connection->GetPosition(&PlayerX, &PlayerY, &PlayerZ);

    int StartZ, EndZ, StepZ;
    if(Up){
        SendByte(SV_CMD_FLOOR_UP);          // 0xBE (190)
        if(PlayerZ == 7){
            // Going to surface (8 -> 7). Send floors [5, 0].
            StepZ = -1; StartZ = 5; EndZ = 0 + StepZ;   // EndZ = -1 (exclusive)
        }else if(PlayerZ > 7){
            // Going up, still underground. Send one floor up.
            StepZ = -1; StartZ = PlayerZ - 2; EndZ = StartZ + StepZ;
        }
    }else{
        SendByte(SV_CMD_FLOOR_DOWN);        // 0xBF (191)
        if(PlayerZ == 8){
            // Going underground (7 -> 8). Send floors [10, 8].
            StepZ = 1; StartZ = 8; EndZ = 10 + StepZ;   // EndZ = 11 (exclusive)
        }else if(PlayerZ > 8 && (PlayerZ + 2) <= 15){
            // Going down, still underground. Send one floor down.
            StepZ = 1; StartZ = PlayerZ + 2; EndZ = StartZ + StepZ;
        }
    }

    if(StartZ != EndZ){
        int MinX = PlayerX - TerminalOffsetX;     // PlayerX - 8
        int MinY = PlayerY - TerminalOffsetY;     // PlayerY - 6
        int MaxX = MinX + TerminalWidth - 1;       // MinX + 17
        int MaxY = MinY + TerminalHeight - 1;      // MinY + 13

        Skip = -1;
        for(int PointZ = StartZ; PointZ != EndZ; PointZ += StepZ){
            int ZOffset = (PlayerZ - PointZ);
            for(int PointX = MinX; PointX <= MaxX; PointX++)
            for(int PointY = MinY; PointY <= MaxY; PointY++){
                SendMapPoint(Connection, PointX + ZOffset, PointY + ZOffset, PointZ);
            }
        }
        SkipFlush(Connection);
    }
}
```

**Key properties:**
- Origin is the player's **current** position (which is the intermediate position during
  `NotifyGo`'s step-by-step update — after the z-step loop adjusted `posx/posy/posz`).
- Floor range:
  - Going up to surface (z=7): floors 5→0 (6 floors)
  - Going up underground (z>7): one floor at `PlayerZ - 2`
  - Going down to underground (z=8): floors 8→10 (3 floors)
  - Going down underground (z>8): one floor at `PlayerZ + 2`
- `ZOffset = PlayerZ - PointZ` shifts the x/y origin per floor (the "floor skip" mechanism —
  higher floors are rendered offset to the northwest).
- Uses the global `Skip` counter + `SkipFlush` to compress empty tiles.

---

## 4. 772 `SendRow`

**Source:** `reference/cipsoft-772/tibia-game-master/src/sending.cc:463-510`

```cpp
void SendRow(TConnection *Connection, int Direction){
    int PlayerX, PlayerY, PlayerZ;
    Connection->GetPosition(&PlayerX, &PlayerY, &PlayerZ);
    int MinX = PlayerX - TerminalOffsetX;    // PlayerX - 8
    int MinY = PlayerY - TerminalOffsetY;    // PlayerY - 6
    int MaxX = MinX + TerminalWidth - 1;      // MinX + 17
    int MaxY = MinY + TerminalHeight - 1;     // MinY + 13

    int StartZ, EndZ, StepZ;
    if(PlayerZ <= 7){
        StepZ = -1; StartZ = 7; EndZ = 0 + StepZ;    // floors 7→0
    }else{
        StepZ = 1; StartZ = PlayerZ - 2;
        EndZ = min<int>(PlayerZ + 2, 15) + StepZ;    // floors (z-2)→min(z+2,15)
    }

    if(Direction == DIRECTION_NORTH){
        SendByte(SV_CMD_ROW_NORTH);   // 0x65 (101)
        MaxY = MinY;                  // only the top row
    }else if(Direction == DIRECTION_EAST){
        SendByte(SV_CMD_ROW_EAST);    // 0x66 (102)
        MinX = MaxX;                  // only the right column
    }else if(Direction == DIRECTION_SOUTH){
        SendByte(SV_CMD_ROW_SOUTH);   // 0x67 (103)
        MinY = MaxY;                  // only the bottom row
    }else if(Direction == DIRECTION_WEST){
        SendByte(SV_CMD_ROW_WEST);    // 0x68 (104)
        MaxX = MinX;                  // only the left column
    }

    Skip = -1;
    for(int PointZ = StartZ; PointZ != EndZ; PointZ += StepZ){
        int ZOffset = (PlayerZ - PointZ);
        for(int PointX = MinX; PointX <= MaxX; PointX++)
        for(int PointY = MinY; PointY <= MaxY; PointY++){
            SendMapPoint(Connection, PointX + ZOffset, PointY + ZOffset, PointZ);
        }
    }
    SkipFlush(Connection);
}
```

**Key properties:**
- Each `SendRow` sends a single row/column of the viewport (the newly exposed edge).
- The row spans **all visible floors** (7→0 on surface, z-2→z+2 underground), not just one z.
- Origin is the player's **current** position (updated step-by-step in `NotifyGo`).
- `ZOffset` shifts x/y per floor (same as `SendFloors`).

---

## 5. 772 `SendMoveCreature` (spectators only)

**Source:** `reference/cipsoft-772/tibia-game-master/src/sending.cc:658-700`

```cpp
void SendMoveCreature(TConnection *Connection,
        uint32 CreatureID, int DestX, int DestY, int DestZ){
    TCreature *Creature = GetCreature(CreatureID);
    int OrigX = Creature->posx, OrigY = Creature->posy, OrigZ = Creature->posz;
    int OrigIndex = GetObjectRNum(Creature->CrObject);
    bool IsVisible = Connection->IsVisible(DestX, DestY, DestZ);
    bool WasVisible = OrigIndex < MAX_OBJECTS_PER_POINT
            && Connection->IsVisible(OrigX, OrigY, OrigZ);
    if(IsVisible && WasVisible){
        SendByte(SV_CMD_MOVE_CREATURE);    // 0x6D (109)
        SendWord(OrigX); SendWord(OrigY); SendByte(OrigZ);
        SendByte(OrigIndex);               // stackpos (RNum)
        SendWord(DestX); SendWord(DestY); SendByte(DestZ);
    }else if(IsVisible){
        SendAddField(Connection, DestX, DestY, DestZ, Creature->CrObject);
    }else if(WasVisible){
        SendDeleteField(Connection, OrigX, OrigY, OrigZ, Creature->CrObject);
    }
}
```

**Called from:** `AnnounceMovingCreature` (`operate.cc:31-57`) — iterates nearby players and
sends `SendMoveCreature` to each. This is the **spectator** path only. The moving player itself
never receives `0x6D` for its own move — it gets `NotifyGo` instead.

**Packet shape:** `0x6D | orig_x:u16 | orig_y:u16 | orig_z:u8 | orig_stack:u8 | dest_x:u16 | dest_y:u16 | dest_z:u8`

Note: 772 has **no `0xFFFF + creature_id` fallback** for stackpos ≥ 10 — it uses `OrigIndex`
directly (which is capped at `MAX_OBJECTS_PER_POINT`). If `OrigIndex >= MAX_OBJECTS_PER_POINT`,
`WasVisible` is false and it falls back to `SendDeleteField`/`SendAddField`.

---

## 6. 772 opcode table

**Source:** `reference/cipsoft-772/tibia-game-master/src/connections.hh:83-133`

| Opcode | Hex | Name | 772 usage |
|--------|-----|------|-----------|
| 100 | 0x64 | `SV_CMD_FULLSCREEN` | Teleport move (full map description) |
| 101 | 0x65 | `SV_CMD_ROW_NORTH` | SendRow north (new top row) |
| 102 | 0x66 | `SV_CMD_ROW_EAST` | SendRow east (new right column) |
| 103 | 0x67 | `SV_CMD_ROW_SOUTH` | SendRow south (new bottom row) |
| 104 | 0x68 | `SV_CMD_ROW_WEST` | SendRow west (new left column) |
| 106 | 0x6A | `SV_CMD_ADD_FIELD` | Add thing to tile |
| 108 | 0x6C | `SV_CMD_DELETE_FIELD` | Remove thing from tile |
| 109 | 0x6D | `SV_CMD_MOVE_CREATURE` | Creature move (spectators only) |
| 190 | 0xBE | `SV_CMD_FLOOR_UP` | Floor change up (SendFloors) |
| 191 | 0xBF | `SV_CMD_FLOOR_DOWN` | Floor change down (SendFloors) |

**Note:** Opcodes 0x65-0x68, 0xBE, 0xBF are the **same** between 772 and 1098. The divergence
is in the **packet structure** (what precedes/follows these opcodes), not the opcode values.

---

## 7. 772 terminal dimensions

**Source:** `reference/cipsoft-772/tibia-game-master/src/connections.cc:219-222`

```
TerminalOffsetX = 8
TerminalOffsetY = 6
TerminalWidth   = 18
TerminalHeight  = 14
```

These match the Rust constants (`MAX_CLIENT_VIEWPORT_X=8, MAX_CLIENT_VIEWPORT_Y=6,
client_viewport_width()=18, client_viewport_height()=14`).

---

## 8. 1098 TFS — `ProtocolGame::sendMoveCreature` (current Rust model)

**Source:** `src/protocolgame.cpp:2827-2894` (TFS 1.4.2)
**Rust:** `crates/tfs-rust-net/src/map_description.rs:671-889` (`send_move_creature_player`)

```cpp
if (creature == player) {
    if (teleport) {
        sendRemoveTileCreature(creature, oldPos, oldStackPos);
        sendMapDescription(newPos);
    } else {
        NetworkMessage msg;
        if (oldPos.z == 7 && newPos.z >= 8) {
            RemoveTileCreature(msg, creature, oldPos, oldStackPos);   // 0x6C
        } else {
            msg.addByte(0x6D);                                        // MOVE CREATURE FOR SELF
            if (oldStackPos < 10) {
                msg.addPosition(oldPos); msg.addByte(oldStackPos);
            } else {
                msg.add<uint16_t>(0xFFFF); msg.add<uint32_t>(creature->getID());
            }
            msg.addPosition(newPos);
        }
        if (newPos.z > oldPos.z) MoveDownCreature(msg, ...);   // 0xBF + floor descriptions
        else if (newPos.z < oldPos.z) MoveUpCreature(msg, ...); // 0xBE + floor descriptions
        // map rows based on oldPos→newPos delta (0x65-0x68)
    }
}
```

**Key differences from 772:**
1. **Sends `0x6D` for the player's own move** — 772 never does this.
2. Floor descriptions use `oldPos` as origin — 772 uses the player's intermediate position
   (after z-step diagonal adjustment).
3. Map rows are based on `oldPos → newPos` delta — 772 generates rows from the z-step's
   diagonal shift + remaining x/y delta.
4. Surface→underground uses `0x6C` (remove) — 772 uses `SendFloors(0xBF)` directly (no remove).

---

## 9. Worked example: stairs down (100,100,7) → (100,101,8)

### 772 `NotifyGo` packets

```
OrigX=100, OrigY=100, OrigZ=7
DestX=100, DestY=101, DestZ=8
DistanceX=0, DistanceY=1, DistanceZ=1 → adjacent path

z-step loop (posz=7 < DestZ=8):
  posx = 99, posy = 99, posz = 8
  SendFloors(Connection, false)     → 0xBF, floor descriptions origin=(99,99,8)

x-step loop (posx=99 < DestX=100):
  posx = 100
  SendRow(Connection, EAST)         → 0x66, column at x=100+8+1=109, y=99-6=93..107

y-step loop (posy=99 < DestY=101):
  posy = 100
  SendRow(Connection, SOUTH)        → 0x67, row at y=100+6+1=107, x=100-8=92..109
  posy = 101
  SendRow(Connection, SOUTH)        → 0x67, row at y=101+6+1=108, x=100-8=92..109
```

**Total: 4 packets** — `0xBF`, `0x66`, `0x67`, `0x67`. No `0x6D`.

### Rust (1098 pattern) packets — **corrected (audit #2)**

`old.z == 7 && new.z >= 8`, so the leading packet is **`0x6C` (remove), not `0x6D`**.
`append_move_down_creature` then emits `0xBF` + a fixed `0x66` (east) + `0x67` (south);
the outer `send_move_creature_player` adds one more `0x67` for the `oy < ny` delta.

```
0x6C  | remove old=(100,100,7) stack=0                 ← SPURIOUS self-packet (not in 772)
0xBF  | floors 8,9,10  origin ox=92,oy=94 offset=-1..  ← tiles = 772's (91,93) after offset ✓
0x66  | east col  x=old.x+9=109, y0=old.y-7=93         ← matches 772 east col ✓
0x67  | south row y=old.y+7=107, x0=92                 ← matches 772 first south row ✓
0x67  | south row y=ny+7=108,   x0=92                  ← matches 772 second south row ✓
```

**Total: 5 packets** — `0x6C`, `0xBF`, `0x66`, `0x67`, `0x67`. The floor + three rows are
**byte-identical to 772**. The single divergence is the leading `0x6C`.

### 772 vs Rust — side by side

| Packet | 772 `NotifyGo` | Rust (1098 path) |
|--------|----------------|------------------|
| self-move | *(none)* | `0x6C` remove ← **spurious** |
| floors 8–10 | `0xBF` origin (91,93) via ZOffset | `0xBF` origin (92,94) + offset ⇒ (91,93) ✓ |
| east col | `0x66` x=109, y=93..106 | `0x66` x=109, y=93..106 ✓ |
| south row #1 | `0x67` y=107 | `0x67` y=107 ✓ |
| south row #2 | `0x67` y=108 | `0x67` y=108 ✓ |

### Desync mechanism (refined)

1. The 772 client receives `0x6C` removing its **own** creature from the old tile. In 772 the
   client tracks its own position from the `NotifyGo` floor/row stream, not from a self
   remove/move packet, so this either desyncs the self-sprite bookkeeping or double-processes
   the transition (see §1 "double-shift").
2. The floor description and all three rows are **correct** — they are not the cause.
3. Removing the leading `0x6C` (and, for other z-changes / same-z moves, the leading `0x6D`)
   should make the stream match 772 for this case.

---

## 10. Worked example: same-z move east (100,100,7) → (101,100,7)

### 772 `NotifyGo` packets

```
OrigX=100, OrigY=100, OrigZ=7
DestX=101, DestY=100, DestZ=7
DistanceX=1, DistanceY=0, DistanceZ=0 → adjacent path

No z-steps (posz == DestZ).
x-step loop (posx=100 < DestX=101):
  posx = 101
  SendRow(Connection, EAST)         → 0x66, column at x=101+8+1=110, y=100-6=94..108

No y-steps.
```

**Total: 1 packet** — `0x66`. No `0x6D`.

### Rust (1098 pattern) packets

```
0x6D  | old=(100,100,7) stack=0 | new=(101,100,7)     ← EXTRA, not in 772
0x66  | east column at x=101+8+1=110, y=100-6=94..108  ← correct
```

**Total: 2 packets** — `0x6D`, `0x66`. ~~The extra `0x6D` is likely ignored by the 772 client
(creature ID == self → skip), so same-z moves work despite the extra packet.~~

> **Correction (2026-07-04 live client debugging).** The §10 assumption was **wrong**.
> The 772 client does NOT track the player's own creature in its tile map. When it receives
> `0x6D` for a self-move, it logs **"MoveCreature has been received for a coordinate where
> there is no creature anymore [bug000017]"** — the source position (player's own tile) has
> no creature entry. The decompile's `NotifyGo` (`cract.cc:1400-1460`) never sends `0x6D`
> for self-moves — the viewport is updated purely via `SendRow` (0x65-0x68), which the client
> interprets as an implicit self-move (shift viewport + update internal position). TVP
> (`protocolgame.cpp:1796`) sends `0x6D` for self-moves (1098 pattern), but TVP was designed
> for OTClient, not the real 772 client. **The decompile is authoritative** for the real
> 772 client. The Rust server now suppresses `0x6D`/`0x6C` for ALL self-moves (same-z AND
> z-changes) for 772, matching the decompile.

---

## 10b. Worked example: hole/ladder straight down (100,100,7) → (100,100,8)

This is the **most common** floor change (holes, ladders, trapdoors) — no x/y delta, pure z.
It is the clearest demonstration that the map payloads already match.

### 772 `NotifyGo`

```
DistanceX=0, DistanceY=0, DistanceZ=1 → adjacent path

z-step (posz=7<8): posx=99, posy=99, posz=8 → SendFloors(down)   0xBF floors 8,9,10 origin (91,93)
x-step (posx=99<100): posx=100         → SendRow(EAST)           0x66 col x=109, y=93..106
y-step (posy=99<100): posy=100         → SendRow(SOUTH)          0x67 row y=107, x=92..109
```

**Total: 3 packets** — `0xBF`, `0x66`, `0x67`. No self-move packet.

Note the counter-intuitive result: even though the player did **not** move in x or y, 772 still
emits an east column and a south row. This is because the z-step shifts `posx/posy` by (−1,−1)
(the diagonal floor offset), and the x/y-step loops then walk those two coordinates back to the
destination, each emitting a `SendRow`.

### Rust (1098 path)

```
0x6C  | remove old=(100,100,7) stack=0                 ← SPURIOUS (not in 772)
0xBF  | floors 8,9,10 origin ox=92,oy=94 (+offset)     ← = 772 (91,93) ✓
0x66  | east col x=old.x+9=109, y0=old.y-7=93          ← = 772 ✓
0x67  | south row y=old.y+7=107, x0=92                 ← = 772 ✓
```

**Total: 4 packets** — `0x6C`, `0xBF`, `0x66`, `0x67`. Floor + both rows match 772 exactly.
**Only** the leading `0x6C` diverges. This case would be **fully fixed** by suppressing the
self-packet — no row/origin changes needed.

---

## 11. Rust code locations to fix

| File | Lines | Function | Issue |
|------|-------|----------|-------|
| `crates/tfs-rust-net/src/map_description.rs` | 671-889 | `send_move_creature_player` | Emits the **spurious self-packet** (`0x6D`/`0x6C`) for 772. **Primary fix**: gate the self-packet on `codec.caps()` era. Rows/floors already match 772. |
| `crates/tfs-rust-net/src/map_description.rs` | 474-568 | `append_move_up_creature` | **Correct as-is** (audit #2): floor tiles + `0x68/0x65` rows are byte-identical to 772 after `offset`. No change needed for pure-vertical/cardinal moves. |
| `crates/tfs-rust-net/src/map_description.rs` | 572-666 | `append_move_down_creature` | **Correct as-is** (audit #2): floor tiles + `0x66/0x67` rows match 772. No change needed. |
| `crates/tfs-rust-net/src/map_description.rs` | 894-911 | `send_move_creature_spectator` | Uses 1098 `0xFFFF + creature_id` fallback for `stack ≥ 10`; 772 has **no fallback** (uses `OrigIndex`, else delete/add). Minor, §16.4. |
| `crates/tfs-rust-core/src/walk/mod.rs` | 950-989 | `emit_move_packet` | Calls `send_move_creature_player` unconditionally (era handled inside via `codec`). |
| `crates/tfs-rust-core/src/walk/mod.rs` | 993-1042 | `emit_teleport_move_packet` | Uses 1098 teleport (remove + map desc); 772 uses `SendFullScreen` (0x64) alone, **no remove**. |
| `crates/tfs-rust-core/src/walk/mod.rs` | 1577-1597 | Segment emission loop | Dispatches `emit_move_packet` vs `emit_teleport_move_packet`. |

---

## 12. 772 `Go` function — height-based floor change (climbing)

**Source:** `reference/cipsoft-772/tibia-game-master/src/cract.cc:379-435`

The 772 walk step executor `Go(DestX, DestY, DestZ)`:
1. Requires `Distance ≤ 1` and `OrigZ == DestZ` (else `throw NOTACCESSIBLE`).
2. Does drunk stagger.
3. If `MovePossible(DestX, DestY, DestZ)` fails, tries height-based climbing:
   - **Go up:** `GetHeight(OrigX, OrigY, OrigZ) >= 24` and tile above is passable → `DestZ -= 1`
   - **Go down:** `GetHeight(DestX, DestY, DestZ+1) >= 24` and tile below is passable → `DestZ += 1`
   - Only for players, non-diagonal moves.
4. Calls `::Move(this->ID, this->CrObject, Dest, -1, false, NONE)` which triggers
   `MoveObject` + `AnnounceMovingCreature` (spectators) + `NotifyGo` (self).

This is the 772 analog of TFS 1.4.2's `resolve_player_move_destination` height check
(`src/game.cpp:807-833`). The Rust implementation is in
`crates/tfs-rust-core/src/walk/walk_tile.rs:112-192`.

---

## 13. 772 `SendFullScreen` (teleport path)

**Source:** `reference/cipsoft-772/tibia-game-master/src/sending.cc:421-460`

```cpp
void SendFullScreen(TConnection *Connection){
    int PlayerX, PlayerY, PlayerZ;
    Connection->GetPosition(&PlayerX, &PlayerY, &PlayerZ);
    int MinX = PlayerX - TerminalOffsetX;
    int MinY = PlayerY - TerminalOffsetY;
    int MaxX = MinX + TerminalWidth - 1;
    int MaxY = MinY + TerminalHeight - 1;

    int StartZ, EndZ, StepZ;
    if(PlayerZ <= 7){
        StepZ = -1; StartZ = 7; EndZ = 0 + StepZ;
    }else{
        StepZ = 1; StartZ = PlayerZ - 2;
        EndZ = min<int>(PlayerZ + 2, 15) + StepZ;
    }

    SendByte(SV_CMD_FULLSCREEN);     // 0x64 (100)
    SendWord(PlayerX); SendWord(PlayerY); SendByte(PlayerZ);

    Skip = -1;
    for(int PointZ = StartZ; PointZ != EndZ; PointZ += StepZ){
        int ZOffset = (PlayerZ - PointZ);
        for(int PointX = MinX; PointX <= MaxX; PointX++)
        for(int PointY = MinY; PointY <= MaxY; PointY++){
            SendMapPoint(Connection, PointX + ZOffset, PointY + ZOffset, PointZ);
        }
    }
    SkipFlush(Connection);
}
```

**Opcode:** `0x64` (100) — same as 1098's map description opcode.
**Shape:** `0x64 | center_x:u16 | center_y:u16 | center_z:u8 | floor descriptions`

The Rust `emit_teleport_move_packet` uses 1098's pattern: `sendRemoveTileCreature` (0x6C) +
`sendMapDescription` (0x64). The 772 teleport path is just `SendFullScreen` (0x64) alone —
no remove packet.

---

## 14. Fix plan (revised after audit #2)

The re-audit (§16) shows the map payload (floor descriptions + edge rows) already matches 772.
The fix is therefore **narrow**: suppress the self-creature packet for the 772 era and adjust
the teleport path. This supersedes the original "Option A full rewrite".

### Phase 0 — Lock behavior with golden tests (do first) — **IMPLEMENTED 2026-07-03**

Golden tests added to `crates/tfs-rust-net/tests/protocol_compat.rs`:

- `mod v1098_floor_change` (5 tests, **passing** — regression guard):
  - `hole_down_in_place_1098_has_self_packet` — (100,100,7)→(100,100,8): asserts `0x6C` + map body, len=20.
  - `ladder_up_in_place_1098_has_self_packet` — (100,100,8)→(100,100,7): asserts `0x6D` + map body, len=31.
  - `stairs_down_diag_1098_has_self_packet` — (100,100,7)→(100,101,8): asserts `0x6C` + 2× `0x67`, len=23.
  - `same_z_east_1098_has_self_packet` — (100,100,7)→(101,100,7): asserts `0x6D` + `0x66`, len=15.
  - `teleport_map_description_1098_starts_with_0x64` — `send_map_description_packet` produces `0x64`, len=22.

- `mod v772_floor_change` (5 tests, **4 failing until Phase 1** — the bug):
  - `hole_down_in_place_no_self_packet` — expects `0xBF` first (no `0x6C`).
  - `ladder_up_in_place_no_self_packet` — expects `0xBE` first (no `0x6D`).
  - `stairs_down_diag_no_self_packet` — expects `0xBF` first (no `0x6C`).
  - `same_z_east_no_self_packet` — expects `0x66` first (no `0x6D`).
  - `teleport_772_map_description_matches_1098_body` — **passing**: map body (`0x64`) is era-independent for empty map.

The 772 tests derive expected bytes by stripping the leading self-packet from the 1098 output
(`assert_772_matches_1098_minus_self_packet`), proving the map body is byte-identical and only
the self-packet diverges (§16.1). All tests use an empty map (`get_tile` → `None`) for deterministic
skip-compression bytes.

### Phase 1 — Suppress the self-creature packet for ALL 772 self-moves (primary fix) — **IMPLEMENTED 2026-07-03, REFINED 2026-07-04**

In `send_move_creature_player` (`map_description.rs`), gate the leading self-packet on the era
for **ALL self-moves** (both z-changes AND same-z). The floor/row emission that follows is
**unchanged** (it already matches 772):

**New capability required.** `ProtocolCaps` (`crates/tfs-rust-common/src/protocol_version.rs:47`)
has no bool that captures this today. Add one — `move_creature_self_packet: bool` — set `true`
for `V1098.caps()` and `false` for `V772.caps()`. This keeps the era branch in the caps matrix,
not scattered `if version` checks (per `tfs-wire-codec.md` "capability-gated" rule).

```rust
let emit_self_packet = codec.caps().move_creature_self_packet; // 772 = false
// z-change branch:
if emit_self_packet {
    if old_pos.z == 7 && new_pos.z >= 8 {
        // 0x6C remove (1098 only)
    } else {
        // 0x6D move (1098 only)
    }
}
// append_move_down/up + outer rows: UNCHANGED (already byte-identical to 772)
// same-z branch: also gated on emit_self_packet (772 suppresses 0x6D for ALL self-moves)
```

> **TVP vs decompile divergence (2026-07-04).** TVP `sendMoveCreature` (`protocolgame.cpp:1796`)
> sends `0x6D` for self-moves (1098 pattern). The decompile's `NotifyGo` (`cract.cc:1400-1460`)
> never sends `0x6D` for self. Live client debugging confirmed the decompile is authoritative:
> the 772 client does NOT track the player's own creature in its tile map, so `0x6D` at the
> player's position triggers "MoveCreature has been received for a coordinate where there is
> no creature anymore [bug000017]". TVP was designed for OTClient (which handles self `0x6D`
> differently), not the real 772 client. **The decompile is authoritative for the real 772
> client.** The Rust server suppresses `0x6D`/`0x6C` for ALL self-moves for 772.
>
> **Earlier same-z revert (2026-07-04) was wrong.** The initial Phase 1 suppressed `0x6D` for
> all self-moves. The user reported "can't move at all" on same-z steps, so we reverted to
> keeping `0x6D` for same-z. This was a misdiagnosis — the real issue was the teleport detection
> (z-changes routing to 0x64 full-screen instead of SendFloors/SendRow), which has since been
> fixed with era-aware `is_adjacent_move`. With both fixes in place (self-packet suppression +
> era-aware teleport detection), same-z moves emit only `SendRow` (0x65-0x68), matching the
> decompile's `NotifyGo`.

### Phase 2 — 772 teleport path — **IMPLEMENTED 2026-07-03**

772 teleport = `SendFullScreen` (`0x64`) alone (§13); 1098 = `0x6C` remove + map description.
Options:
- **(a)** In `emit_teleport_move_packet` (`walk/mod.rs`), skip the `sendRemoveTileCreature` step
  when `!codec.caps().move_creature_self_packet` and emit only the map description (already
  `0x64`). Simplest.
- **(b)** Route the teleport self-packet suppression through the codec for symmetry with Phase 1.

Prefer (a) — the remove is the only divergent piece; the map description is identical.

### Phase 3 — Verify row ordering on combined diagonal+z moves (§16.3) — **VERIFIED NO-OP 2026-07-03**

For moves with **both** a z-change and a leftover x **and** y delta (e.g. diagonal stair-steps),
772 orders rows *x-steps then y-steps*, while the Rust path emits append's fixed `0x66/0x67`
then the outer `N/S`-before-`E/W` rows. If a golden test for such a move mismatches on ordering,
reorder the outer row emission for the 772 era to match `NotifyGo` (x-loop before y-loop). If no
in-game move actually produces a leftover in both axes after the diagonal shift, this phase is a
no-op — verify with a test before writing code.

### Phase 4 — (Optional) spectator `0xFFFF` fallback (§16.4) — **IMPLEMENTED 2026-07-03**

Low priority; only affects tiles with ≥10 stacked objects. Give `send_move_creature_spectator`
an era-aware path: 772 uses `OrigIndex` directly and, when `OrigIndex ≥ MAX_OBJECTS_PER_POINT`,
falls back to delete+add rather than the `0xFFFF + id` form.

### Out of scope (track separately)

- `EarliestWalkTime` diagonal-3× cost + `Beat` quantization (§2.3) — walk *timing*, not desync.
  Verify against Rust walk-speed code in a separate task.
- Container-close / trade-cancel side effects in `NotifyGo` (§2.3) — behavior parity, not wire.

### Verification commands

```
cargo test -p tfs-rust-net --test protocol_compat
cargo test -p tfs-rust-core walk
cargo clippy -p tfs-rust-net -p tfs-rust-core --all-targets -- -D warnings
```

Then manual smoke test with a real 772 client: hole down, ladder up, stairs (diagonal), rope,
teleport scroll — confirm no viewport desync on each.

---

## 15. References

| What | 772 decompile | Rust |
|------|---------------|------|
| Player self-move | `cract.cc:1400-1460` `NotifyGo` | `map_description.rs:671-889` `send_move_creature_player` |
| SendFloors | `sending.cc:517-573` | `map_description.rs:474-568` `append_move_up_creature` / `572-666` `append_move_down_creature` |
| SendRow | `sending.cc:463-510` | `map_description.rs:738-800` (inline in `send_move_creature_player`) |
| SendMoveCreature (spectators) | `sending.cc:658-700` | `map_description.rs:894-911` `send_move_creature_spectator` |
| SendFullScreen (teleport) | `sending.cc:421-460` | `walk/mod.rs:993-1042` `emit_teleport_move_packet` |
| Go (walk step + climbing) | `cract.cc:379-435` | `walk/walk_tile.rs:112-192` `resolve_player_move_destination` |
| Opcodes | `connections.hh:83-133` | `codec/v772.rs` |
| Terminal dims | `connections.cc:219-222` | `protocol_constants.rs:13-26` |

---

## 16. Audit #2 findings (2026-07-03 re-audit)

This section records the second-pass verification against the live Rust code and 772 source.
It corrects the earlier worked examples and narrows the root cause.

### 16.1 Map payload already matches 772 (corrects §9/§10 claims)

The original doc treated `append_move_up_creature` / `append_move_down_creature` as if they
emitted **only** the floor description. They do not — each also emits the diagonal-shift edge
rows (verified in `map_description.rs`):

- `append_move_down_creature` (`~572-666`): `0xBF` floors, then `0x66` east column
  (`x = old.x + 9`, `y0 = old.y - 7`), then `0x67` south row (`y = old.y + 7`, `x0 = old.x - 8`).
- `append_move_up_creature` (`~474-568`): `0xBE` floors, then `0x68` west column, then `0x65`
  north row.

The floor-description origin difference is only apparent. 772 `SendFloors` reads the
**shifted** position (`posx-1, posy-1` after the z-step) and applies `ZOffset = PlayerZ - PointZ`.
Rust `get_floor_description` keeps `old_pos` as origin but adds `offset = origin_z - point_z`
to **both** `tx` and `ty` (`map_description.rs:230-231`). The two formulations are algebraically
equal:

```
772:  tile = (PlayerX_shifted - 8) + ZOffset,  ZOffset = PlayerZ - PointZ
Rust: tile = (old.x - 8) + offset,             offset  = origin_z - point_z
```

With `PlayerX_shifted = old.x - 1` and `PlayerZ = origin_z - 1` (down by one), both reduce to the
same absolute tile coordinate. Verified numerically for the down-in-place, stairs-down-diagonal,
and up-to-surface cases — **all floor and row tiles are byte-identical**.

**Consequence:** the earlier "wrong floor origin" and "missing south row" claims are withdrawn.
The map stream is correct; only the leading self-creature packet is spurious.

### 16.2 `0x6C` vs `0x6D` (corrects §9 heading)

`send_move_creature_player` sends `0x6C` (remove tile creature), **not** `0x6D`, whenever
`old.z == 7 && new.z >= 8` (surface→underground). §9's original trace mislabeled this as `0x6D`.
For all other z-changes and same-z moves it sends `0x6D`. Both are absent from 772's `NotifyGo`.

### 16.3 Row-ordering divergence on combined diagonal+z moves (new)

772 `NotifyGo` orders the walk-back as **x-steps then y-steps** (east/west before south/north).
The Rust path emits `append_move_*`'s fixed column+row first, then the outer loop's rows in
**N/S before E/W** order. For a move that leaves a non-zero delta in **both** x and y after the
diagonal z-shift, the row *sequence* differs even though the set of rows is the same. Because
`SendRow`/`0x65-0x68` carry **no coordinates** (the client applies each to the next viewport
edge), sequence can matter. This is the only remaining wire divergence beyond the self-packet —
Phase 3 of the fix plan gates it behind a golden test (it may be unreachable in practice, since
a single walk step rarely leaves both axes non-zero after the (−1,−1) shift).

### 16.4 Spectator `0xFFFF` fallback divergence (new, minor)

`send_move_creature_spectator` (`map_description.rs:894-911`) always emits `0x6D` and uses the
1098 `0xFFFF + creature_id` form when `old_stack >= 10`. 772 `SendMoveCreature` (§5) has **no**
such fallback: it writes `OrigIndex` (the RNum stackpos) directly, and when
`OrigIndex >= MAX_OBJECTS_PER_POINT` it treats the creature as not-was-visible and sends
`SendDeleteField` (`0x6C`) / `SendAddField` (`0x6A`) instead. Only matters on tiles with ≥10
stacked objects; low priority (Phase 4).

### 16.5 Items confirmed accurate in the original doc

- `NotifyGo` structure, loop order, and the "no self-packet" thesis (§2) — confirmed against
  `cract.cc:1400-1464`.
- `SendFloors` / `SendRow` / `SendFullScreen` / `SendMoveCreature` reproductions (§3-§5, §13) —
  confirmed against `sending.cc` (note: the real code reads `Connection->TerminalOffsetX` etc.
  as per-connection fields, not the global constants shown; the values match §7).
- Opcode table (§6) and terminal dims (§7) — confirmed.
- `Skip`/`SkipFlush` tile compression — the Rust `skip` counter + `skip,0xFF` flush in
  `get_floor_description` matches 772's `SendMapPoint`/`SkipFlush` semantics; no gap.

### 16.6 Non-wire detail noted but not packet-relevant

`NotifyGo` opens with `MoveChainCreature(this, DestX, DestY)` (`cract.cc:1407`), which only
updates the 16×16 spatial-hash bucket (`crmain.cc:1058`). It has no effect on packet emission
and is the 772 analog of the Rust spectator grid update — not part of this desync.
