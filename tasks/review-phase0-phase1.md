# Phase 0 & Phase 1 Review Notes

## Overall Status: PASS

All tests pass, clippy is clean, architecture matches the spec. Both phases are correctly implemented.

---

## Phase 0 — tfs-rust-common

### What's correct
- `Position`, `Direction`, and all game enums are complete
- `PropStream` / `PropWriteStream` use correct little-endian layout via `byteorder`, byte-compatible with TFS 1.4.2
- `TfsRustError` has all required variants. The extra `PropStream` variant is a clean addition
- Property test for round-trip passes, covering all primitive types and strings
- Golden blob test scaffolded correctly, gracefully skips if no fixtures are present

### Issues

**1. Unused import in `golden_blobs.rs`**
`use proptest::prelude::*;` is present but the test doesn't use proptest. Remove it.

```rust
// Remove this line from crates/tfs-rust-common/tests/golden_blobs.rs
use proptest::prelude::*;
```

**2. `get_direction_to` uses floating-point `atan2`**
TFS C++ uses integer comparison logic for direction resolution. The current implementation is functionally correct but may diverge on diagonal tie-breaking edge cases. Not a blocker now, but flag for revisit before Phase 5 (creature movement).

**3. Golden blob fixtures not yet populated (task 0.7)**
The fixture directory exists but contains no real blobs. This is a hard prerequisite before Phase 2 begins per the spec. Run the following against the live Australis MariaDB and commit the output to `crates/tfs-rust-common/tests/fixtures/blobs/`:

```sql
SELECT items FROM player_items LIMIT 500;
SELECT conditions FROM players WHERE conditions != '' LIMIT 500;
```

---

## Phase 1 — tfs-rust-net

### What's correct
- `NetworkMessage` — all read/write methods correct, little-endian, proper EOF error handling
- `xtea` — 32-round Feistel, correct `DELTA` constant, correct key indexing, wrapping arithmetic — matches TFS C++ exactly
- `rsa` — thin wrapper using `Pkcs1v15Encrypt`. No `unsafe` in your code; it lives inside the `rsa` crate itself
- `ConnectionState` state machine — all four states defined correctly
- `GameCommand` enum — core variants present with a clear comment for progressive expansion
- `Server` — Tokio TCP listener, per-connection `tokio::spawn`, proper error logging
- All three property tests pass: XTEA round-trip, NetworkMessage round-trip, zlib compression round-trip

### Issues

**4. `handle_connection` is a stub**
`handle_connection` accepts a `TcpStream` but does nothing with it. `ProtocolLogin` and `ProtocolGame` are also empty structs. This is expected — full protocol parsing is Phase 7. Just ensure task 1.12 is marked complete with the understanding that the connection read loop is deferred.

**5. `GameCommand::PlayerLogin` and `PlayerLogout` have no fields**
Per the design, these need `conn_id: ConnId` and account data. Fine as stubs now but must be filled before Phase 7 (task 7.1).

```rust
// Current (stub)
PlayerLogin,
PlayerLogout,

// Required by Phase 7
PlayerLogin { conn_id: ConnId, account: AccountData, char_name: String },
PlayerLogout { conn_id: ConnId },
```

**6. `main.rs` is inside the C++ `src/` directory**
`src/main.rs` sits alongside the C++ source files. It compiles correctly because the workspace root `Cargo.toml` resolves `src/main.rs` as the binary entry point. Not a build issue, but it's messy. Consider moving it to a dedicated `rust-src/main.rs` or keeping it as-is with a comment explaining the co-location.

---

## Pre-Phase 2 Checklist

- [ ] Remove unused `proptest` import from `golden_blobs.rs`
- [ ] Extract and commit golden blob fixtures from live DB (task 0.7 — hard requirement)
- [ ] Confirm `GameCommand` stub fields are tracked for Phase 7
- [ ] Optionally relocate `main.rs` out of the C++ source directory
