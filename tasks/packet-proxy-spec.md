# Packet Proxy Tool

## Purpose

A transparent TCP proxy that sits between OTClient and the Rust server, decrypts both directions using the XTEA key captured during the handshake, and logs every packet with opcode, direction, size, and hex dump. Eliminates guesswork from OTClient error messages.

```
OTClient → proxy:7171 (login) → Rust server:7172
OTClient → proxy:7172 (game)  → Rust server:7173
                ↓
         logs/packets.log
```

The real server moves to ports 7172/7173. The proxy listens on 7171/7172 (what OTClient connects to).

---

## Location

`tools/packet-proxy/` — standalone Cargo binary in the workspace.

```toml
# tools/packet-proxy/Cargo.toml
[package]
name = "packet-proxy"
version = "0.1.0"
edition = "2021"

[dependencies]
tfs-rust-net = { workspace = true }
tfs-rust-common = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = "0.3"
hex = "0.4"
```

---

## What it needs to do

### Phase 1 — Raw TCP passthrough with hex logging

Before any decryption, just forward bytes in both directions and log every TCP chunk with direction and hex dump. This alone is useful for verifying framing.

```
[C→S] 14 bytes: 0e 00 43 a2 ad 3d 01 02 00 4a 04 4a 04 00
[S→C] 14 bytes: 0c 00 59 98 1f 63 06 00 1f a3 42 00 00 03
```

### Phase 2 — XTEA key extraction and decryption

The proxy intercepts the game port first packet (RSA block), decrypts it using the same `rsa::decrypt` from `tfs-rust-net`, and extracts the 16-byte XTEA key. From that point on, all subsequent packets in both directions are decrypted before logging.

Key extraction happens once per connection in `parse_first_client_packet` — the proxy already has this code available from `tfs-rust-net`.

### Phase 3 — Opcode annotation

After decryption, read the first byte of each payload as the opcode and annotate it with a human-readable name from a lookup table.

```
[S→C] opcode=0x17 (LoginSuccess)     38 bytes: 17 01 00 00 00 32 00 ...
[S→C] opcode=0x0A (PendingState)      1 bytes: 0a
[S→C] opcode=0x0F (EnterWorld)        1 bytes: 0f
[S→C] opcode=0x64 (MapDescription) 2118 bytes: 64 64 00 c8 00 07 ...
[C→S] opcode=0x65 (WalkNorth)         1 bytes: 65
```

### Phase 4 — Map description parser (optional, high value)

Parse `0x64` packets and pretty-print the tile grid so you can see exactly what the server is sending vs what OTClient expects.

```
[S→C] 0x64 MapDescription center=(100,200,7)
  floor z=7:
    (92,194): ground=106 [anim] items=[] creatures=[]
    (93,194): SKIP×3
    (96,194): ground=106 [anim] items=[2148] creatures=[player:1234]
    ...
  floor z=6: all skip
```

---

## Connection flow

```
1. OTClient connects to proxy:7171 (login port)
2. Proxy connects to server:7172
3. Proxy forwards all bytes bidirectionally, logging each chunk
4. Login completes — character list sent back to OTClient

5. OTClient connects to proxy:7172 (game port)
6. Proxy connects to server:7173
7. Proxy intercepts first packet, extracts XTEA key
8. All subsequent packets decrypted before logging, re-encrypted before forwarding
```

---

## Implementation structure

```
tools/packet-proxy/src/
├── main.rs          — CLI args, bind listeners, spawn connection handlers
├── connection.rs    — bidirectional TCP proxy with logging
├── handshake.rs     — RSA decrypt + XTEA key extraction from first game packet
├── decrypt.rs       — XTEA decrypt/encrypt wrappers using tfs-rust-net
├── opcodes.rs       — opcode → name lookup table (server and client opcodes)
└── logger.rs        — structured packet log output (stdout + optional file)
```

---

## CLI usage

```bash
# Start proxy (server must be on 7172/7173)
cargo run -p packet-proxy -- \
  --login-listen 127.0.0.1:7171 \
  --login-upstream 127.0.0.1:7172 \
  --game-listen 127.0.0.1:7172 \
  --game-upstream 127.0.0.1:7173 \
  --key key.pem \
  --log logs/packets.log
```

---

## Log format

Each line:

```
[timestamp] [direction] opcode=0xXX (Name) size=N bytes
  hex: XX XX XX XX XX XX XX XX  XX XX XX XX XX XX XX XX
  hex: ...
```

For map packets, optionally expand tile contents on subsequent lines.

---

## What this solves

| Problem | How proxy helps |
|---------|-----------------|
| `invalid thing id (0)` | See exact bytes of `0x64` packet, find where item ID 0 appears |
| `eof reached after 0x17` | See exact byte count of `0x17` packet vs expected 38 bytes |
| XTEA inner length bug | Compare raw encrypted bytes vs expected framing |
| Unknown packet ordering issues | See full sequence in both directions with timestamps |
| Regression testing | Capture a known-good session from C++ TFS, replay against Rust server |

---

## Reuse from existing crates

- `tfs-rust-net::rsa::decrypt` — RSA block decryption for key extraction
- `tfs-rust-net::xtea` — XTEA encrypt/decrypt
- `tfs-rust-net::game_frame::read_sized_payload` — TCP framing
- `tfs-rust-net::adler::adler_checksum` — checksum verification
- `tfs-rust-common::protocol_opcodes` — opcode constants for annotation
