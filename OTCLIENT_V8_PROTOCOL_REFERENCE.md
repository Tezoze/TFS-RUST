# OTClient v8 — protocol reference (game transport, login vs game, opcodes)

This document consolidates how **OTClient v8** in this tree builds and parses packets, aligned with the **TFS / tfs-rust** mental model (transport, inner length, login vs game). Sources: `src/framework/net/protocol.cpp`, `inputmessage.cpp`, `outputmessage.cpp`, `src/client/protocolgamesend.cpp`, `src/client/protocolgameparse.cpp`, `src/client/protocolcodes.h`, `modules/gamelib/protocollogin.lua`.

Unless stated otherwise, behaviour matches **client version 1098** with stock **`modules/game_features/features.lua`** (`updateFeatures(1098)`). Servers may send extra feature bits via login / OT extensions; this doc describes the **default 1098** feature set.

---

## 0. Protocol 1098 — stock OTClient feature snapshot

Derived from **`modules/game_features/features.lua`** (all `if version >= X` branches that fire for `1098`):

| Area | 1098 default |
|------|----------------|
| **Protocol / crypto** | `GameLoginPacketEncryption`, `GameProtocolChecksum`, `GameChallengeOnLogin`, `GameMessageSizeCheck`, **`GameSessionKey`** (1074+), **`GameAuthenticator`** |
| **Login / world** | **`GameLoginPending`** (981+) — opcode **`0x0A`** uses **`parsePendingGame`**, not `parseLogin` |
| **Versions** | **`GameClientVersion`**, **`GamePreviewState`**, **`GameContentRevision`** (1071+), **`GameNewSpeedLaw`** (981+) |
| **Map / combat** | **`GameLooktypeU16`**, **`GamePVPMode`**, **`GameAttackSeq`**, **`GameEnvironmentEffect`** (910+) — extra per-tile `u16` when not Tibia 12 |
| **Not enabled** for stock 1098 | **`GamePrey`** (1100+), **`GameSequencedPackets`**, **`GameTibia12Protocol`**, **`GamePlayerStateU32`** (1200+), **`GamePacketSizeU32`** (not toggled in `features.lua`), **`GamePacketCompression`** (not toggled in `features.lua`) |
| **Not enabled** unless you add it | **`GameNewWalking`** — stock `features.lua` never enables it; **no** per-tile ground-speed `u16` + blocking `u8` in **`setTileDescription`** unless you enable this feature |

**Inner / outer sizes:** for default 1098, `m_bigPackets` is false → **inner `u16`** after XTEA decrypt, **outer `u16`** TCP length; no sequenced-packet sequence dword instead of Adler.

**Login (character list) port:** still **`modules/gamelib/protocollogin.lua`** — includes RSA, optional second RSA for authenticator, optional OGL blob when **`GameOGLInformation`** (1061+) is on for 1098.

---

## 1. Transport (after game login, both directions)

Applies once **`enableXteaEncryption()`** and **`enableChecksum()`** are active on the game `Protocol` (see `ProtocolGame::sendLoginPacket` in `protocolgamesend.cpp`: checksum + send, then XTEA for subsequent messages). Both are on for stock **1098** (`GameLoginPacketEncryption` + `GameProtocolChecksum`).

### 1.1 TCP framing

- **Outer length:** `u16` LE (or `u32` LE if “big packets” / `enableBigPackets()` — not enabled for default 1098 feature set in `modules/game_features/features.lua`).
- That length is the size of the **next block** (the “body” the client reads after the 2-byte header). Implementation: `Protocol::internalRecvHeader` → `readSize(m_bigPackets)` then `read(remainingSize, …)`.

### 1.2 Body layout (typical encrypted game message)

Order in **`Protocol::send`** (`protocol.cpp`, non-`rawPacket`):

1. **`xteaEncrypt`** (if `m_xteaEncryptionEnabled`): encrypts the payload block (see §1.3).
2. **`writeChecksum`** (if `m_checksumEnabled` and not sequenced): writes **Adler-32** (`stdext::adler32`) over the message bytes at the current write position — see `OutputMessage::writeChecksum()`.
3. **`writeMessageSize`**: writes the outer size field (prepended in the header region).

Checksum is **Adler-32**, not CRC32 (`outputmessage.cpp` / `inputmessage.cpp`).

### 1.3 XTEA + inner length (matches TFS `NetworkMessage` crypto layout)

**Encrypt (outgoing)** — `Protocol::xteaEncrypt`:

- Calls `writeMessageSize(m_bigPackets)` first so the **plaintext starts with an inner size field** (`u16` or `u32` depending on `m_bigPackets`; **`u16` for 1098**).
- Pads with **zero** bytes (`addPaddingBytes` default) until length is a **multiple of 8**.
- Encrypts with XTEA (delta `0x61C88647`, 32 rounds, key schedule as in source).

**Decrypt (incoming)** — `Protocol::xteaDecrypt`:

- Ciphertext length must be **divisible by 8**.
- Decrypts in place.
- Then:  
  `decryptedSize = bigPackets ? (getU32() + 4) : (getU16() + 2)`  
  i.e. after decrypt, the first **2** (or **4**) bytes are the inner length; the **total** decrypted logical size is **inner + 2** (or **inner + 4** for big). The implementation adjusts `messageSize` with `sizeDelta` (see `protocol.cpp`).

If your Rust decoder’s **inner length** or padding does not match this, you get **“inner length too small”** (or similar) until the layout matches OTClient’s `xteaDecrypt` / `xteaEncrypt`.

**1098 + `GameMessageSizeCheck` (841+):** on the **first** server→client game packet, after decrypt, OTClient checks that the leading **`u16`** size equals **`getUnreadSize()`** (`protocolgame.cpp` `ProtocolGame::onRecv`). With stock 1098, **`GamePacketSizeU32`** is off — use **`u16`**, not `u32`.

### 1.4 Compression / sequenced packets

- `internalRecvData`: if checksum `peekU32() == 0`, optional **zlib** path (`m_compression` / decompress). Stock **1098** does **not** enable **`GamePacketCompression`** in `features.lua`; treat compression as off unless your pipeline enables it.
- **`GameSequencedPackets`** (≥**1200** in `features.lua`): uses `writeSequence` instead of Adler checksum in `Protocol::send`; **not used for 1098**.

---

## 2. Login port (character list) — not the same as game transport

- Implemented in Lua: **`modules/gamelib/protocollogin.lua`** (`ProtocolLogin:sendLoginPacket`).
- **First client message:** plain structure (RSA + optional fields), **not** wrapped in the game XTEA+Adler stream above.
- Server replies with login opcodes (MOTD, character list, session key, errors) — **`modules/gamelib/protocollogin.lua`** parse side.

Use this path for **login listeners** / smoke tests that only talk to the **login** TCP port.

---

## 3. Game port — first messages (before XTEA stream)

### 3.1 Server → client: challenge

- Opcode **`GameServerChallenge` = 31 (`0x1F`)** — `ProtocolGame::parseChallenge` reads `u32` timestamp + `u8` random, then **`sendLoginPacket(timestamp, random)`**.

### 3.2 Client → server: first real game packet (RSA, then enable XTEA)

- **`ProtocolGame::sendLoginPacket`** in **`protocolgamesend.cpp`**:
  - Opcode **`ClientPendingGame` = 10 (`0x0A`)** — not `ClientEnterAccount` (that is login-only).
  - Cleartext header for **1098**: OS `u16`, protocol `u16`, **`u32` client version** (`GameClientVersion`), **`u16` content revision only** (`GameContentRevision` — unlike login Lua, no extra `u16(0)` here), **`u8` preview** (`GamePreviewState`).
  - RSA plaintext: leading `u8(0)`, **16-byte XTEA key** + `u8` GM flag (`GameLoginPacketEncryption`), then **`GameSessionKey`**: session key string + character name (1098 uses session path, not account/password in this block), **`GameChallengeOnLogin`** tail, **`OTCv8` + client build `u16`**, etc.
- After send: **`enableChecksum()`**, **`enableXteaEncryption()`**; **`enableCompression()`** only if `GamePacketCompression` (off in stock 1098).

**Note:** “RSA at offset 16” depends on **exact feature set** (client version, content revision, preview, etc.). Count bytes from `sendLoginPacket` for your build rather than assuming a fixed offset.

### 3.3 `sendEnterGame` — not the first packet

- **`ProtocolGame::sendEnterGame`**: single byte **`ClientEnterGame` = 15 (`0x0F`)**.
- Called **later** from `Game::processPendingGame()` after the server drives pending state — **no RSA**, not the first game message.

---

## 4. Server → client — initial burst (TFS-style order vs OTClient names)

Your **enqueue** order (first byte = game opcode) maps to **OTClient `Proto::GameServerOpcodes`** in **`protocolcodes.h`** and parsers in **`protocolgameparse.cpp`**:

| Order | Opcode (hex) | Decimal | OTClient enum | Parser / role |
|------:|-------------|--------|---------------|----------------|
| 1 | `0x17` | 23 | **`GameServerLoginSuccess`** | **`parseLogin`** — `u32` player id, `u16` server beat, **`GameNewSpeedLaw`** (981+) three doubles, `u8` can report bugs, optional fields from **1054** / **1058** / **`GameIngameStore`** (1080+) for 1098; **no** Tibia 12-only tail. **Not** `GameServerCreateOnMap` (creature on map is **`0x6A` / 106**). |
| 2 | `0x0A` | 10 | **`GameServerLoginOrPendingState`** | For **1098**: **`parsePendingGame`** (`GameLoginPending` is on). Older protocols without login-pending use **`parseLogin`**. |
| 3 | `0x0F` | 15 | **`GameServerEnterGame`** | **`parseEnterGame`** — triggers `processEnterGame` / `processGameStart` when appropriate. |
| 4 | `0x64` | 100 | **`GameServerFullMap`** | **`parseMapDescription`** → **`setMapDescription`** / **`setTileDescription`** — full map strip. |
| 5 | `0x82` | 130 | **`GameServerAmbient`** | **`parseWorldLight`** — `u8` intensity, `u8` color. |
| 6 | `0xA7` | 167 | **`GameServerPlayerModes`** | **`parsePlayerModes`** — fight `u8`, chase `u8`, safe `u8`, plus **`u8` PVP mode** (`GamePVPMode` is on for 1098). |

**TFS naming** (e.g. “self appear”, `GetMapDescription`) may differ from enum names; use the **hex opcode** as the stable key when comparing to **OTClient** and **tfs-rust** `protocol_opcodes.rs`.

### 4.1 Map `0x64` — full format in OTClient (1098)

- **`parseMapDescription`**: reads position, then **`setMapDescription`** → per-floor **`setFloorDescription`** → **`setTileDescription`** (tiles, items, creatures, skip bytes).
- For **1098** stock features, each tile (before the thing stack) includes **`GameEnvironmentEffect`**: an extra **`u16`** when **`GameTibia12Protocol`** is off (true for 1098). **`GameNewWalking`** is **off** by default — **no** ground-speed `u16` + blocking `u8` unless you enable that feature in OTClient.
- A **stub** that only sends opcode + position + `0xFF 0xFF` end marker will **not** satisfy this parser; expect **black map / wrong view** until the body matches **`setTileDescription`** / **`getThing`** logic (see `protocolgameparse.cpp` around `setMapDescription` / `setTileDescription`).

---

## 5. Client → server — examples (OTClient `Proto::ClientOpcodes`)

First byte is opcode. **`protocolcodes.h`** is the authoritative checklist for this client; compare with **`tfs-rust-common`** `protocol_opcodes.rs` `client::`.

| Opcode (hex) | Decimal | OTClient name | Notes |
|-------------|---------|---------------|--------|
| `0x0F` | 15 | **`ClientEnterGame`** | Single byte, after pending; TFS in-game `default` no-op (tfs-rust: `GamePacket::EnterGame`). |
| `0x1D` | 29 | **`ClientPing`** | Same byte as **`tfs-rust-common`** `client::PING_BACK` — Rust names follow **TFS** `playerReceivePingBack` / `playerReceivePing`, not OTClient enum labels. |
| `0x1E` | 30 | **`ClientPingBack`** | Same byte as **`tfs-rust-common`** `client::PING`. |
| `0x65`–`0x68` | 101–104 | **`ClientWalkNorth`…`ClientWalkWest`** | Cardinal walks |
| `0x6A`–`0x6D` | 106–109 | **`ClientWalkNorthEast`…** | Diagonals |
| `0x6F`–`0x72` | 111–114 | **`ClientTurnNorth`…`ClientTurnWest`** | Turns |
| `0x96` | 150 | **`ClientTalk`** | Say / talk |
| `0x32` | 50 | **`ClientExtendedOpcode`** | OTClient extended opcode (same numeric as first “game” opcode range) |

Many more (trade, move item, store, …) — see **`protocolcodes.h`** `enum ClientOpcodes`. Prey opcodes apply only with **`GamePrey`** (1100+), not stock **1098**.

---

## 6. Practical debugging

| Symptom | What to align |
|--------|----------------|
| **Inner length too small** after decrypt | OTClient **`xteaDecrypt`**: inner `u16`/`u32`, padding to 8, **Adler** scope vs your Rust parser. |
| **Ping mismatch** | `0x1D` / `0x1E` vs server ping mode; **`GameClientPing`** changes ping/back behavior in **`parseMessage`** (`protocolgameparse.cpp`). |
| **Black map** | **`0x64`** body must match **`parseMapDescription` / `setTileDescription`**, not a minimal stub. |
| **Login vs game** | Login port = **`protocollogin.lua`**; game port = **`sendLoginPacket`** + **`protocol.cpp`** transport. |

---

## 7. Source file index

| Topic | Location |
|-------|-----------|
| TCP read, checksum, XTEA, decrypt | `src/framework/net/protocol.cpp` |
| Adler checksum write/read | `src/framework/net/outputmessage.cpp`, `inputmessage.cpp` |
| Game login packet + RSA | `src/client/protocolgamesend.cpp` — `sendLoginPacket`, `sendEnterGame` |
| Connect / challenge / first recv | `src/client/protocolgame.cpp` |
| Server opcode dispatch | `src/client/protocolgameparse.cpp` — `parseMessage` |
| Opcode constants | `src/client/protocolcodes.h` |
| Login server (Lua) | `modules/gamelib/protocollogin.lua` |
| Feature flags (1098, etc.) | `modules/game_features/features.lua` |

---

## 8. External checklist (Rust)

Use **`crates/tfs-rust-common/src/protocol_opcodes.rs`** under **`client::`** as the **server-side parse checklist**; cross-check each value against **`protocolcodes.h`** here. Differences in **extended opcode ranges** or **version-gated fields** are common — always gate on protocol version and enabled features like OTClient’s `g_game.getFeature(...)`.

---

## 9. What your server must implement (OTClient source anchors)

This is the **binary-compatible** behaviour behind §1.3 and §4.1: your Rust (or other) game server should mirror these functions, not only the prose in §1–4.

### 9.1 Send path (client → wire) — `Protocol::send`

Implementation: `src/framework/net/protocol.cpp` (`Protocol::send`, `xteaEncrypt`).

**Order of operations** (non-`rawPacket`):

1. **`xteaEncrypt`** — writes **inner** length via `OutputMessage::writeMessageSize`, pads to **8-byte** boundary, then XTEA-encrypts the block whose first words include that inner length (`xteaEncrypt` reads from `getDataBuffer() - (big ? 4 : 2)`).
2. **`writeChecksum`** — **Adler-32** over **`m_buffer + m_headerPos`** for **`m_messageSize`** bytes (i.e. the **encrypted payload only**, before checksum and outer size are prepended). See `OutputMessage::writeChecksum()`.
3. **`writeMessageSize`** — prepends **outer** `u16` LE (or `u32` if big packets) = total bytes to follow in that TCP chunk (see `OutputMessage::writeMessageSize()`).

So on the wire (default `m_bigPackets == false`):  
**`[ u16 outerLen ][ u32 adler32 ][ XTEA ciphertext… ]`**  
where the ciphertext decrypts to plaintext beginning with **inner `u16`/`u32`** + payload, and **inner logical length** after decrypt is **`innerU16 + 2`** or **`innerU32 + 4`** (`xteaDecrypt`).

**Padding:** `xteaEncrypt` adds `8 - (size % 8)` bytes via **`addPaddingBytes`** — **zero** padding by default (not random). Matches **1098** `m_bigPackets == false` → inner **`u16`**.

### 9.2 Receive path (wire → client) — `Protocol::internalRecvData`

Implementation: same file — **`readChecksum` then `xteaDecrypt`**.

- After the **outer** length read, the **body** is: **`u32` Adler** (unless `peekU32() == 0` compression escape) then **XTEA data**.
- **`InputMessage::readChecksum`**: consumes **`getU32()`**, then **`adler32(m_buffer + m_readPos, getUnreadSize())`** — checksum covers **everything after** the 4-byte checksum field (the encrypted block).
- **`xteaDecrypt`**: requires encrypted length **% 8 == 0**, decrypts, then reads inner length at start of plaintext and trims (`decryptedSize = getU16()+2` or `getU32()+4`).

Your **encoder** (server sending to OTClient) should invert this exactly: build plaintext with inner length, pad, XTEA, prepend Adler over ciphertext, prepend outer length.

### 9.3 Non-stub **`0x64` (`GameServerFullMap`)** — what “full map” means in code

OTClient does **not** stop at opcode + position + `0xFF 0xFF`; it runs the **map strip** pipeline:

| Step | Function | File (approx.) |
|------|----------|----------------|
| Opcode **`100`** | `parseMapDescription` | `protocolgameparse.cpp` |
| Position + dimensions | `setMapDescription` | `protocolgameparse.cpp` |
| Per-floor grid | `setFloorDescription` | `protocolgameparse.cpp` |
| Per-tile stack | **`setTileDescription`** | `protocolgameparse.cpp` |
| Skip / end-of-tile | `peekU16() >= 0xFF00` → `getU16() & 0xFF` | **`setTileDescription`** |
| Things | **`getThing`** → **`getItem`** / **`getCreature`** / static text | `protocolgameparse.cpp` |

**Feature-gated fields inside tiles (1098 stock):** **`GameEnvironmentEffect`** → extra **`u16`** per tile (when not Tibia 12). **`GameLooktypeU16`**, **`GamePlayerMounts`**, addons, etc. are on for 1098 — see **`setTileDescription`**, **`getCreature`**, **`getOutfit`**. **`GameNewWalking`** is **not** enabled by stock `features.lua`; do **not** send ground-speed + blocking before the stack unless your client enables that feature.

To get a **visible map**, your server must emit the same **tile / item / creature** binary as **`setTileDescription` + `getThing`** expect, not a minimal stub.

### 9.4 Quick file:line map

| Concern | Location |
|--------|----------|
| Send order: XTEA → checksum → outer size | `protocol.cpp` `Protocol::send` |
| Inner length + padding + XTEA rounds | `protocol.cpp` `xteaEncrypt` / `xteaDecrypt` |
| Adler scope (send) | `outputmessage.cpp` `writeChecksum` |
| Adler scope (recv) | `inputmessage.cpp` `readChecksum` |
| Full map parse | `protocolgameparse.cpp` `parseMapDescription`, `setMapDescription`, `setFloorDescription`, **`setTileDescription`**, **`getThing`** |
