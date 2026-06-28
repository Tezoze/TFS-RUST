---
trigger: model_decision
description: How to add wire functions, packets, and encoders behind the ProtocolCodec seam (772 / 1098).
globs: ["crates/tfs-rust-net/**/*.rs", "crates/tfs-rust-common/src/protocol*.rs"]
---

# Wire Codec (772 / 1098)

Complements `@.cursor/rules/TFS-packets.mdc`. Architecture: `docs/PROTOCOL_VERSIONING.md` §4.

## Transport is capability-gated

Adler32 checksum, pre-login `0x1F` challenge, XTEA slack (`-4` vs `-6`) follow **`ProtocolCaps`** — not hardcoded 1098 assumptions.

## Add a server→client send

1. **Neutral struct** — `XxxWire` with max-width fields (`u64` exp, `u32` cap). Core fills it; never pre-narrow.
2. **Core** — build `XxxWire` at existing emission sites (`game_world.rs`, `walk.rs`, …). **No** `msg.write_*` in core.
3. **Trait** — add `ProtocolCodec::write_xxx(&self, msg, &XxxWire)`.
4. **Impls** — `Codec1098` (current behavior) and `Codec772` when layout differs. Branch lives in codec only.
5. **Opcode** — version-keyed table in `protocol_opcodes.rs`. Never inline hex at call sites.
6. **C++ ref** — 1098: repo-root `protocolgame.cpp` / `networkmessage.cpp`; **772: `gameserver/src/` only** (same filenames — never repo-root `src/`, never `tibia-game-master`).
7. **Test** — golden bytes in `tests/protocol_compat.rs`, one module per version.

## Add a client→server handler

1. Parse raw bytes in `game_parse.rs` (version-keyed opcode dispatch) → **`GamePacket`** semantic variant.
2. Core handles the typed variant — core never reads raw packet bytes.

## File placement

| Piece | Location |
|-------|----------|
| `ProtocolVersion`, `ProtocolCaps` | `tfs-rust-common` |
| `ProtocolCodec`, `Codec772`, `Codec1098` | `tfs-rust-net/src/codec/` |
| Per-version encoders | `codec/v772.rs`, `codec/v1098.rs` |
| Neutral wire structs | net (or common if shared with tests) |

## Do not

- Add version checks in `tfs-rust-core`
- Route DB `item_blob` through the codec
- Treat OTCv8 quirks as generic 1098 — flag separately (`docs/OTCLIENT_INFO.md`)
- Guess 772 layouts — confirm against **`gameserver/src/` only**; cite file + function
- Use `tibia-game-master` or repo-root `src/` for **772** packet/protocol references
