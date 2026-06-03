---
trigger: always_on
---

# Threading Model (Game Thread vs I/O Threads)

TFS Rust uses a hybrid threading model: single-threaded game simulation + Tokio async I/O.

## Architecture Overview

```
┌─────────────────┐         ┌──────────────────┐
│  Tokio I/O      │         │   Game Thread    │
│  (tfs-rust-net) │  mpsc   │  (tfs-rust-core) │
│                 │ ──────> │                  │
│ - Parse packets │         │ - GameWorld      │
│ - Encryption    │         │ - SlotMap<>s     │
│ - DB queries    │         │ - Map            │
│ - Send buffers  │         │ - Game loop      │
└─────────────────┘         └──────────────────┘
```

## Game Thread Ownership (Single-Threaded)

**Game thread owns all simulation state:**
- `GameWorld` and all fields
- `SlotMap<CreatureId, CreatureKind>` — creatures
- `SlotMap<ItemId, Item>` — items
- `Map` — tiles and spatial data
- `player_by_name: HashMap<String, CreatureId>`
- `conn_to_creature: HashMap<ConnId, CreatureId>`

**These are NOT `Send + Sync`** — they live only on the game thread.

## I/O Threads (Tokio Tasks)

**I/O threads handle:**
- Network packet parsing and encoding
- RSA decryption, XTEA encryption
- Database queries (SQLx async)
- Sending encoded packets to TCP sockets

**I/O threads NEVER:**
- Access `GameWorld` directly
- Mutate SlotMap entities
- Touch game simulation state

## Bridge Pattern (mpsc Channels)

Communication uses Tokio `mpsc` channels:

```rust
// tfs-rust-net: I/O thread parses packet
let command = GameCommand::PlayerMove { conn_id, direction };
game_tx.send(command).await?;

// tfs-rust-core: Game thread processes command
while let Some(cmd) = game_rx.recv().await {
    match cmd {
        GameCommand::PlayerMove { conn_id, direction } => {
            world.handle_player_move(conn_id, direction);
        }
    }
}
```

## Document Thread Ownership

Mark game-thread-only data structures:

```rust
pub struct GameWorld {
    /// GAME THREAD ONLY — insert/remove from I/O threads must not be added.
    pub player_by_name: HashMap<String, CreatureId>,

    /// GAME THREAD ONLY — paired with `player_by_name`.
    pub player_by_guid: HashMap<u32, CreatureId>,
}
```

## Database Queries

Spawn DB queries from game thread, await results via oneshot channels:

```rust
// Game thread spawns query
let (tx, rx) = tokio::sync::oneshot::channel();
tokio::spawn(async move {
    let player = tfs_rust_db::load_player(&pool, guid).await;
    let _ = tx.send(player);
});

// Later in game loop
if let Ok(Ok(player)) = rx.try_recv() {
    world.add_player(player);
}
```

## Output Queuing

Game thread queues outgoing packets, I/O thread drains and sends:

```rust
// Game thread: Queue packet
world.pending_outgoing
    .entry(conn_id)
    .or_default()
    .push(encoded_packet);

// Game thread: Flush each tick
world.flush_output_buffers(&out_registry);

// I/O thread: OutRegistry sends to TCP sockets
```

## Tokio Spawn Rules

**Use `tokio::spawn` for:**
- Network I/O (accept connections, read/write sockets)
- Database queries
- CPU-bound work that needs parallelism (rare)

**NEVER use `tokio::spawn` for:**
- Mutating `GameWorld`
- Game logic that must run serially
- Anything touching SlotMap entities

## Send + Sync Boundaries

**Game state types are NOT Send:**
```rust
// GameWorld, SlotMap, Map: no Send/Sync bounds
// They live only on game thread
```

**Commands are Send:**
```rust
#[derive(Debug)]
pub enum GameCommand {
    PlayerMove { conn_id: ConnId, direction: Direction },
    // IDs are Send, payloads are owned
}
```

## Performance Notes

- Single-threaded game loop avoids lock contention
- Tokio handles thousands of concurrent connections efficiently
- Channel sends are cheap (lockless MPSC)
- Game thread never blocks on I/O