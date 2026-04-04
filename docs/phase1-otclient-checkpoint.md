# Phase 1 — Manual OTClient connection checkpoint (task 1.13)

This checkpoint catches TCP framing and client compatibility issues that unit tests alone will not find.

## Prerequisites

- [OTClient](https://github.com/edubart/otclient) built for protocol **10.98** (this project targets 10.98, not 8.6).
- Rust toolchain; project root at the workspace root.

## Procedure

1. Start a TCP listener that keeps connections open (protocol parsing is still minimal):

   ```bash
   cargo run -p tfs-rust-net --example tcp_smoke
   ```

2. Read the terminal line that says which address was bound (the example tries `127.0.0.1:7171`, then `[::1]:7171`, then `0.0.0.0:7171`).

3. In OTClient, set the **same host family** the server printed:
   - If bound to `127.0.0.1`, use host **`127.0.0.1`** (not `localhost`, which may resolve to IPv6 `::1` and miss an IPv4-only listener).
   - If bound to `[::1]`, use **`localhost`** or **`::1`**.
   - Port is always **7171** for this smoke test.

4. Connect. You should see **`[tfs-rust-net] accepted TCP connection from …`** in the terminal when the TCP handshake succeeds.

5. Expected today:
   - TCP stays open while the example runs (server reads until the client closes).
   - A **full** login handshake (RSA decrypt + character list) is **not** expected until the login stack is wired from `tfs_rust_core::run()` and protocol handlers; track that under Phase 5.

6. When the integrated binary exposes the real login pipeline, repeat this checklist and confirm:
   - RSA decrypt succeeds on the login message.
   - Character list or expected error packet is returned (no silent disconnect before application data).

## If the client “times out” connecting

- **No `accepted TCP connection` line** — the SYN never reached this process. Common causes: wrong host (`localhost` vs `127.0.0.1` / IPv6), wrong port (must be **7171** for this example), firewall blocking `0.0.0.0`, or another program already using 7171 (bind would usually fail; on some systems behaviour differs).
- **OTClient on Windows, server in WSL** — `127.0.0.1` in OTClient is the Windows loopback, not the Linux listener. Use the WSL VM IP from `ip addr` / `hostname -I`, or run OTClient on the same OS as the server.
- **Account / protocol screen** — smoke test only proves TCP; login UI may still wait for game protocol that is not implemented yet.

## Framing / compatibility notes

Document issues discovered during manual runs:

| Date       | Client     | Symptom | Cause / fix |
|------------|------------|---------|-------------|
| *(none yet)* |            |         |             |

## Related code

- `ConnectionState::PendingLogin` — task 1.6b; queue/drop rules in `pending_login.rs`.
- `Server::accept_loop` / `handle_connection` — `crates/tfs-rust-net/src/server.rs`.
