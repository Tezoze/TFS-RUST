# Design Document: TFS Rust Rust Engine

## Overview

TFS Rust is a ground-up rewrite of the Australis TFS 1.4.2 C++ game server (87,826 lines) into idiomatic Rust. The design preserves 100% Tibia 8.6 protocol compatibility, 100% Lua API compatibility, and full MariaDB schema compatibility, while eliminating entire classes of C++ bugs through Rust's ownership model and type system.

The engine is organized as a Cargo workspace of six crates with strict dependency boundaries enforced at compile time. The concurrency model separates game state mutation (single-threaded, tick-driven) from I/O (Tokio async runtime), communicating via typed `mpsc` channels.

### Key Design Decisions

1. **Single-threaded game state + Tokio async I/O**: All `GameWorld` mutations happen on one logical game thread driven by a 50ms `tokio::time::interval`. Network I/O, database queries, and timers run on the Tokio runtime and communicate with the game thread via `tokio::sync::mpsc` channels. This matches the TFS threading model while making the boundary explicit and safe.

2. **Generational arena for entity storage**: `slotmap::SlotMap<CreatureId, CreatureKind>` replaces raw `Creature*` pointers. Dangling references to dead creatures are impossible — a stale `CreatureId` returns `None` on lookup.

3. **Enum dispatch replaces C++ virtual inheritance**: `CreatureKind { Player, Monster, Npc }` and `ConditionData { Damage, Speed, Outfit, ... }` replace polymorphic hierarchies. Pattern matching is zero-cost and exhaustive.

4. **No unsafe except FFI boundaries**: The only `unsafe` blocks permitted are those required by `mlua` (LuaJIT FFI) and the `rsa` crate. All game logic is safe Rust.

5. **Prepared statements only**: `sqlx` with compile-time-checked queries replaces all string-concatenated SQL. SQL injection is structurally impossible.

6. **ContainerKind enum**: `ContainerKind { Regular, DepotChest, DepotLocker, Inbox, StoreInbox }` replaces the C++ `Container` subclass hierarchy.

7. **Native-first monster AI**: Standard monster AI (pathfinding, target selection, basic melee) runs entirely in Rust. The Lua `onThink` creature event is only fired for monsters that have explicitly registered the event handler — matching TFS behavior and keeping FFI crossings to a minimum during the hot creature-think loop.

8. **Async-safe extended opcodes**: Lua `PacketHandler` callbacks for extended opcodes may only dispatch work to Tokio via `db.asyncQuery` or `GameCommand` channel messages. Synchronous DB calls are blocked from extended opcode handlers to prevent stalling the 50ms tick.

9. **Incremental Lua GC**: `GameWorld::tick()` calls `lua.gc(LuaGCMode::Step, step_size)` every 5 ticks to spread LuaJIT garbage collection across multiple ticks, preventing stop-the-world GC spikes under heavy script load.

10. **Golden Blob test fixtures**: Before Phase 2, 1,000+ real item and condition blobs are extracted from the live Australis MariaDB and committed as test fixtures. `PropStream` correctness is validated against real production data, not synthetic data.


---

## Architecture

### Crate Dependency Graph

```
tfs-rust-common  (no tfs-rust-* deps)
    ↑
    ├── tfs-rust-content  (common only)
    ├── tfs-rust-db       (common only)
    ├── tfs-rust-net      (common only)
    └── tfs-rust-core     (common + content + db + lua)
            ↑
        tfs-rust-lua      (common + core)
```

The dependency graph is a DAG with `tfs-rust-common` at the root. `tfs-rust-core` is the integration crate that wires all subsystems together. `tfs-rust-lua` sits above `tfs-rust-core` because Lua bindings need access to game types, but the core game logic does not depend on Lua directly — it fires events through a trait interface that `tfs-rust-lua` implements.

### Concurrency Model

```
┌─────────────────────────────────────────────────────────────┐
│  Tokio Runtime (multi-threaded)                             │
│                                                             │
│  ┌──────────────┐   ┌──────────────┐   ┌────────────────┐  │
│  │ TCP Accept   │   │ I/O Tasks    │   │ DB Tasks       │  │
│  │ Loop         │   │ (per conn)   │   │ (sqlx pool)    │  │
│  └──────┬───────┘   └──────┬───────┘   └───────┬────────┘  │
│         │                  │                   │            │
│         └──────────────────┼───────────────────┘            │
│                            │ mpsc::Sender<GameCommand>       │
└────────────────────────────┼────────────────────────────────┘
                             ▼
┌─────────────────────────────────────────────────────────────┐
│  Game Thread (single logical thread)                        │
│                                                             │
│  tokio::time::interval(50ms)                                │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  GameWorld::tick()                                   │   │
│  │  1. Drain mpsc channel (process network commands)    │   │
│  │  2. process_creature_walk()                          │   │
│  │  3. process_creature_think()                         │   │
│  │  4. process_condition_ticks()                        │   │
│  │  5. process_decay()                                  │   │
│  │  6. process_light_cycle()                            │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

The game thread owns `GameWorld` exclusively. No `Arc<Mutex<GameWorld>>` is needed — the channel boundary enforces the ownership transfer. Database results are returned via `oneshot` channels and processed at the start of the next tick.

### Workspace Layout

```
tfs-rust/
├── Cargo.toml                    # Workspace root
├── src/main.rs                   # Entry point
└── crates/
    ├── tfs-rust-common/            # Position, enums, PropStream, errors
    ├── tfs-rust-content/           # OTBM, OTB, XML loaders
    ├── tfs-rust-db/                # sqlx pool, player_data, market, migrations
    ├── tfs-rust-net/               # TCP server, protocols, XTEA/RSA, NetworkMessage
    ├── tfs-rust-lua/               # mlua bindings, script loader, EventCallback
    └── tfs-rust-core/              # GameWorld, creatures, combat, items, world, events
```


---

## Components and Interfaces

### tfs-rust-common

The foundation crate. Zero dependencies on other TFS Rust crates.

**Key types:**
- `Position { x: u16, y: u16, z: u8 }` — with `distance_to`, `get_direction_to`, `offset`, range checks
- `PropStream` / `PropWriteStream` — binary serialization for item/condition DB blobs
- All game enums: `Direction`, `CombatType`, `ConditionType`, `SkullType`, `ZoneType`, `ReturnValue`, `ItemGroup`, `WeaponType`, `Skill`, `PlayerSex`, `WorldType`, etc.
- `TfsRustError` — unified error type via `thiserror`

### tfs-rust-net

TCP server, connection state machine, protocol parsers/senders, encryption.

**Key components:**
- `Server` — Tokio TCP listener, spawns a `Connection` task per accepted socket
- `Connection` — state machine: `Handshake → Login | Game | Status`
- `NetworkMessage` — byte buffer with typed `read_*` / `write_*` methods (little-endian)
- `ProtocolLogin` — RSA decrypt first message, return character list
- `ProtocolGame` — XTEA decrypt/encrypt, parse 50+ incoming opcodes, send 60+ outgoing packets
- `ProtocolStatus` — XML status response
- `xtea` module — encrypt/decrypt 8-byte blocks with 128-bit key
- `rsa` module — decrypt 128-byte RSA login block

**Channel interface to game thread:**
```rust
pub enum GameCommand {
    PlayerMove { conn_id: ConnId, direction: Direction },
    PlayerSay { conn_id: ConnId, speak_type: SpeakType, text: String },
    PlayerUseItem { conn_id: ConnId, pos: Position, stack_pos: u8, item_id: u16 },
    PlayerAttack { conn_id: ConnId, target_id: u32 },
    PlayerLogin { conn_id: ConnId, account: AccountData, char_name: String },
    PlayerLogout { conn_id: ConnId },
    ExtendedOpcode { conn_id: ConnId, opcode: u8, buffer: Vec<u8> },
    // ... all 50+ client packet types
}
```

### tfs-rust-db

Async MariaDB access via `sqlx`. All queries are prepared statements.

**Key components:**
- `DbPool` — wraps `sqlx::MySqlPool`, configurable min/max connections
- `player_data` module — `load_player`, `save_player`, `load_items`, `save_items`
- `market` module — create/accept/cancel/browse offers
- `migrations` module — schema version check and migration runner
- `queries/` — typed query functions for accounts, players, guilds, houses, bans

**Retry policy:** Transient connection errors trigger up to 3 retries with exponential backoff (100ms, 200ms, 400ms) before returning `Err`.

### tfs-rust-content

Stateless content loaders. Reads data files and returns typed registries. No game state.

**Key components:**
- `OtbmLoader` — parses OTBM binary map format into `MapData { tiles, spawns, houses, towns, waypoints }`
- `OtbLoader` — parses OTB binary item database into `ItemDatabase`
- `ItemDatabase` — merges OTB + `items.xml` into `HashMap<u16, ItemType>`
- `MonsterDatabase` — parses `monsters/` XML into `HashMap<String, MonsterType>`
- `VocationDatabase`, `OutfitDatabase`, `MountDatabase`, `GroupDatabase` — XML loaders
- All loaders run concurrently via `tokio::join!` at startup

### tfs-rust-core

The game engine. Owns `GameWorld` and all game state.

**Key components:**
- `GameWorld` — central struct owning all entities, map, scheduler, event dispatchers
- `creature/` — `CreatureBase`, `Player`, `Monster`, `Npc`, `Party`, `Guild`
- `combat/` — `execute`, `CombatParams`, `ConditionData`, `MatrixArea`, `Weapon`, `Spell`
- `item/` — `Item`, `Container`, `DecayManager`
- `world/` — `Map`, `Tile`, `QTree`, `House`, `SpawnManager`, `Pathfinding`
- `events/` — `CreatureEvents`, `GlobalEvents`, `MoveEvents`, `Actions`, `TalkActions`
- `chat.rs` — channel system
- `config.rs` — `ConfigManager` (loads `config.lua` via mlua)
- `wildcard_tree.rs` — trie for player name prefix lookup

### tfs-rust-lua

LuaJIT scripting bridge via `mlua`. Depends on `tfs-rust-core` to access game types.

**Key components:**
- `LuaState` — initializes LuaJIT runtime, registers all 30+ userdata metatables
- `bindings/` — one file per Lua class (`player.rs`, `creature.rs`, `item.rs`, etc.)
- `loader.rs` — discovers and loads scripts in TFS order: `lib/ → events/ → scripts/ → monster/ → npc/`
- `callbacks.rs` — `EventCallback` registry: `HashMap<EventType, Vec<LuaFunction>>`
- `NpcScriptInterface` — NPC-only Lua functions available only within NPC script contexts

**Error isolation:** Every Lua call is wrapped in `lua.pcall(...)` (mlua's error handling). Errors are caught, logged with script name and line number, and the game tick continues.


---

## Data Models

### Entity Storage

```rust
// tfs-rust-core/src/game.rs
pub struct GameWorld {
    pub creatures: SlotMap<CreatureId, CreatureKind>,
    pub items: SlotMap<ItemId, Item>,
    pub map: Map,
    pub scheduler: Scheduler,
    pub events: EventDispatcher,
    pub config: Arc<Config>,
    pub db: DbPool,
    pub lua: LuaState,
    // Concurrent lookups accessible from I/O tasks:
    pub player_by_name: DashMap<String, CreatureId>,
    pub player_by_guid: DashMap<u32, CreatureId>,
}

slotmap::new_key_type! {
    pub struct CreatureId;
    pub struct ItemId;
}

pub enum CreatureKind {
    Player(Player),
    Monster(Monster),
    Npc(Npc),
}
```

### Creature Model

```rust
pub struct CreatureBase {
    pub id: u32,
    pub name: String,
    pub position: Position,
    pub direction: Direction,
    pub health: i32,
    pub max_health: i32,
    pub outfit: Outfit,
    pub speed: i32,
    pub base_speed: i32,
    pub skull: SkullType,
    pub conditions: Vec<ActiveCondition>,
    pub registered_events: Vec<String>,
    pub walk_queue: VecDeque<Direction>,
    pub follow_target: Option<CreatureId>,
    pub attack_target: Option<CreatureId>,
    pub master: Option<CreatureId>,
    pub damage_map: HashMap<u32, (i64, u32)>, // guid → (damage, ticks)
}

pub struct Player {
    pub base: CreatureBase,
    pub guid: u32,
    pub account_id: u32,
    pub group: GroupId,
    pub vocation_id: u16,
    pub sex: PlayerSex,
    pub level: u32,
    pub experience: u64,
    pub mana: i32,
    pub max_mana: i32,
    pub soul: u32,
    pub stamina: u16,
    pub blessings: u8,
    pub inventory: PlayerInventory,
    pub skills: PlayerSkills,
    pub economy: PlayerEconomy,
    pub social: PlayerSocial,
    pub storage: HashMap<u32, i32>,
    pub open_containers: HashMap<u8, ContainerRef>,
    pub protocol: Option<ProtocolGameHandle>,
}

pub struct Monster {
    pub base: CreatureBase,
    pub monster_type: Arc<MonsterType>,
    pub spawn: Option<SpawnId>,
    pub target_list: Vec<CreatureId>,
    pub friend_list: Vec<CreatureId>,
    pub is_idle: bool,
    pub walk_to_spawn: bool,
}

pub struct Npc {
    pub base: CreatureBase,
    pub npc_type: Arc<NpcType>,
    pub focus_creature: Option<CreatureId>,
    pub shop_items: Vec<ShopInfo>,
    pub script_interface: NpcScriptInterface,
}
```

### Condition Model

```rust
pub struct ActiveCondition {
    pub condition_type: ConditionType,
    pub id: ConditionId,
    pub sub_id: u32,
    pub ticks: i32,
    pub end_time: i64,
    pub is_buff: bool,
    pub aggressive: bool,
    pub data: ConditionData,
}

pub enum ConditionData {
    Damage {
        intervals: Vec<IntervalInfo>,
        total_damage: i32,
        min_damage: i32,
        max_damage: i32,
    },
    Speed {
        speed_delta: i32,
        formula: Option<SpeedFormula>,
    },
    Outfit { outfit: Outfit },
    Light { light: LightInfo, internal_ticks: i32 },
    Regeneration {
        health_ticks: u32, health_gain: u32,
        mana_ticks: u32, mana_gain: u32,
    },
    Soul { soul_ticks: u32, soul_gain: u32 },
    Attributes {
        skills: [i32; SKILL_COUNT],
        stats: [i32; STAT_COUNT],
        special_skills: [i32; SPECIAL_SKILL_COUNT],
        disable_defense: bool,
    },
    SpellCooldown { spell_id: u32 },
    SpellGroupCooldown { group_id: u32 },
    Generic,
}
```

### Item Model

```rust
pub struct Item {
    pub type_id: u16,
    pub count: u16,
    pub action_id: u16,
    pub unique_id: u16,
    pub attributes: ItemAttributes,
    pub decay_state: DecayState,
    pub parent: ParentRef,
}

pub struct ItemAttributes {
    pub attack: Option<i16>,
    pub defense: Option<i16>,
    pub extra_defense: Option<i16>,
    pub armor: Option<i16>,
    pub hit_chance: Option<i8>,
    pub shoot_range: Option<u8>,
    pub charges: Option<u16>,
    pub duration: Option<i32>,
    pub text: Option<String>,
    pub description: Option<String>,
    pub writer: Option<String>,
    pub written_date: Option<i64>,
    pub name: Option<String>,
    pub custom: HashMap<String, AttributeValue>,
}

pub enum ContainerKind {
    Regular,
    DepotChest { max_items: u32 },
    DepotLocker { depot_id: u16 },
    Inbox,
    StoreInbox,
}
```

### Combat Model

```rust
pub struct CombatParams {
    pub combat_type: CombatType,
    pub effect: MagicEffect,
    pub distance_effect: ShootEffect,
    pub blocked_by_armor: bool,
    pub blocked_by_shield: bool,
    pub aggressive: bool,
    pub area: Option<MatrixArea>,
    pub conditions: Vec<ConditionParams>,
    pub value_callback: Option<LuaCallback>,
    pub tile_callback: Option<LuaCallback>,
    pub target_callback: Option<LuaCallback>,
}

pub struct MatrixArea {
    pub data: Vec<Vec<bool>>,
    pub center_x: u32,
    pub center_y: u32,
}
```

### Map Model

```rust
pub struct Map {
    pub root: QTreeNode,
    pub width: u16,
    pub height: u16,
    pub houses: HouseManager,
    pub spawns: SpawnManager,
    pub towns: HashMap<u32, Town>,
    pub waypoints: HashMap<String, Position>,
}

pub struct Tile {
    pub position: Position,
    pub ground: Option<Item>,
    pub items: Vec<Item>,
    pub creatures: Vec<CreatureId>,
    pub flags: TileFlags,
    pub zone: ZoneType,
    pub house_id: Option<u32>,
}
```


### Network Protocol Model

```rust
// Connection state machine
pub enum ConnectionState {
    Handshake,
    Login(ProtocolLogin),
    Game(ProtocolGame),
    Status(ProtocolStatus),
    Closed,
}

// Per-connection XTEA state
pub struct XteaState {
    pub key: [u32; 4],
    pub enabled: bool,
}

// NetworkMessage — owns a Bytes buffer
pub struct NetworkMessage {
    buf: BytesMut,
    read_pos: usize,
}
```

### Lua Binding Model

```rust
// Each game object exposed to Lua is a thin wrapper holding a CreatureId/ItemId
// rather than a raw pointer. The wrapper resolves the ID against the GameWorld
// on each method call, returning a Lua error if the entity no longer exists.

pub struct PlayerRef(pub CreatureId);
pub struct CreatureRef(pub CreatureId);
pub struct ItemRef(pub ItemId);
pub struct TileRef(pub Position);

impl mlua::UserData for PlayerRef {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("getName", |lua, this, ()| {
            let world = lua.app_data_ref::<GameWorldHandle>().unwrap();
            let name = world.with_player(this.0, |p| p.base.name.clone())
                .ok_or_else(|| mlua::Error::runtime("player no longer exists"))?;
            Ok(name)
        });
        // ~120 more methods...
    }
}
```

---

## Subsystem Designs

### Game Loop and Scheduler

The game loop runs as a dedicated Tokio task that owns `GameWorld`. It drives a `tokio::time::interval(Duration::from_millis(50))` and processes one tick per interval fire.

```
tick N:
  1. recv_all(cmd_rx)          // drain all pending GameCommands from network tasks
  2. process_creature_walk()   // advance walk queues, fire step events
  3. process_creature_think()  // monster AI (native Rust), NPC think, condition ticks
  4. process_decay()           // advance decay timers, transform/remove items
  5. process_light_cycle()     // update world light level
  6. lua_gc_step()             // incremental LuaJIT GC step (every 5 ticks)
  7. flush_output_buffers()    // send all queued packets to connections
```

**Native-first monster AI:** `process_creature_think()` runs the full Rust AI loop (target selection, pathfinding, melee attack) for every monster unconditionally. The Lua `onThink` creature event is only dispatched via FFI for monsters whose `registered_events` list contains `"onThink"` — typically only bosses and monsters with custom spell scripts. This keeps FFI crossings proportional to the number of scripted monsters, not the total monster count.

**Incremental Lua GC:** Every 5 ticks, `lua.gc(LuaGCMode::Step, 200)` runs a bounded GC step. This prevents LuaJIT's stop-the-world collector from accumulating stale `PlayerRef`/`CreatureRef` wrapper objects held by poorly-written scripts and causing multi-second latency spikes.

The `Scheduler` wraps `tokio::time::sleep_until` + a `mpsc::Sender<GameCommand>`. Lua's `addEvent(fn, delay, ...)` schedules a `tokio::spawn` that sleeps then sends a `GameCommand::LuaCallback` to the game thread.

### Network Protocol Pipeline

```
TCP stream
  → read_header() [4 bytes: length + checksum]
  → read_body(length)
  → xtea::decrypt(body, key)   [if XTEA enabled]
  → flate2::decompress(body)   [if compression enabled]
  → parse_opcode(body[0])
  → dispatch to GameCommand variant
  → mpsc::send(cmd_tx)
```

Outgoing direction:
```
GameWorld produces OutputMessage
  → flate2::compress(body)     [if compression enabled]
  → xtea::encrypt(body, key)   [if XTEA enabled]
  → prepend_header(length + checksum)
  → tokio::io::AsyncWriteExt::write_all(stream)
```

### XTEA Implementation

XTEA is a 64-round Feistel cipher operating on 8-byte blocks with a 128-bit key. The Rust implementation is ~50 lines of safe code:

```rust
pub fn encrypt(data: &mut [u8], key: &[u32; 4]) {
    assert!(data.len() % 8 == 0);
    for chunk in data.chunks_exact_mut(8) {
        let mut v0 = u32::from_le_bytes(chunk[0..4].try_into().unwrap());
        let mut v1 = u32::from_le_bytes(chunk[4..8].try_into().unwrap());
        let mut sum: u32 = 0;
        for _ in 0..32 {
            v0 = v0.wrapping_add(
                (((v1 << 4) ^ (v1 >> 5)).wrapping_add(v1))
                ^ (sum.wrapping_add(key[(sum & 3) as usize]))
            );
            sum = sum.wrapping_add(DELTA);
            v1 = v1.wrapping_add(
                (((v0 << 4) ^ (v0 >> 5)).wrapping_add(v0))
                ^ (sum.wrapping_add(key[((sum >> 11) & 3) as usize]))
            );
        }
        chunk[0..4].copy_from_slice(&v0.to_le_bytes());
        chunk[4..8].copy_from_slice(&v1.to_le_bytes());
    }
}
```

### Quadtree Spatial Index with Sector Cache

The map uses a quadtree (`QTreeNode`) for `get_spectators(pos, range)` queries, matching the TFS `QTreeNode` structure. Each leaf covers a 32×32 tile sector. The tree is built once at map load and mutated only when creatures move.

```rust
pub enum QTreeNode {
    Branch {
        children: Box<[Option<QTreeNode>; 4]>,
        bounds: Rect,
    },
    Leaf {
        creatures: Vec<CreatureId>,
        sector: SectorCoord,
        cached_spectators: Option<Vec<CreatureId>>, // invalidated on creature enter/leave
    },
}
```

`get_spectators` traverses the tree, collecting `CreatureId` values from all leaf nodes whose bounds intersect the query rectangle. This is O(k log n) where k is the result count.

**Sector spectator cache:** Each leaf node caches its creature list. The cache is invalidated only when a creature enters or leaves that sector — not on every query. Since `get_spectators` is called on every creature move, every magic effect, and every item change, this eliminates redundant quadtree traversals in busy areas and is the single largest performance win available in the hot path.

### A* Pathfinding

The pathfinder mirrors the TFS `AStarNodes` implementation: a priority queue of open nodes, a closed set, and a cost function that accounts for tile walkability, creature blocking, and diagonal movement cost.

```rust
pub fn pathfind(
    map: &Map,
    from: Position,
    to: Position,
    params: &PathParams,
) -> Option<Vec<Direction>> {
    // AStarNodes priority queue (BinaryHeap<AStarNode>)
    // Returns None if no path found within MAX_NODES limit
}
```

### Extended Opcode Async Safety

Extended opcode handlers run Lua `PacketHandler` callbacks on the game thread. To prevent a heavy DB query from stalling the 50ms tick, the following constraint is enforced:

- Extended opcode Lua handlers **may not** call synchronous DB functions (`db.query`, `db.storeQuery`)
- They **must** use `db.asyncQuery` / `db.asyncStoreQuery`, which dispatch to Tokio and return results via `GameCommand` channel on the next tick
- The `NpcScriptInterface` and `PacketHandler` Lua environments expose only the async DB variants

```
ExtendedOpcode received
  → Lua PacketHandler callback invoked
  → if heavy work needed: db.asyncQuery(sql, callback)
      → tokio::spawn(async { result = query.await; cmd_tx.send(GameCommand::LuaAsyncResult) })
  → game tick continues immediately
  → next tick: GameCommand::LuaAsyncResult processed, callback invoked with result
```

### OutputMessage Queue with Backpressure

Each connection maintains a bounded `VecDeque<OutputMessage>` send queue. The game thread enqueues packets during the tick; the Tokio I/O task drains the queue asynchronously.

```rust
pub struct ConnectionSendQueue {
    queue: VecDeque<OutputMessage>,
    max_size: usize,          // configurable, default 256 messages
    bytes_pending: usize,
}
```

**Backpressure rules:**
- If `bytes_pending` exceeds the configured limit, stale visual-only packets (map description chunks, magic effects, creature movement) are dropped in favour of critical packets (stats, inventory, text messages)
- Large sends (e.g. full depot open) are split across multiple ticks via a `pending_chunks` queue
- If a connection's queue is full for more than 3 consecutive ticks, the connection is closed

### Hot Reload System

Script hot reload is a first-class feature, not an afterthought. `world.reload(ReloadType)` is implemented in Phase 9 alongside the event system.

```rust
pub enum ReloadType {
    Scripts,
    Monsters,
    Npcs,
    Actions,
    TalkActions,
    MoveEvents,
    GlobalEvents,
    CreatureEvents,
    All,
}
```

The reload sequence uses `arc-swap` to atomically replace the script registry:
1. Load new scripts into a fresh `LuaState` clone
2. Validate all scripts load without error
3. `arc_swap.store(Arc::new(new_state))` — atomic swap, zero downtime
4. Old state is dropped after all in-flight Lua calls complete

### Player Name Map Safety

`DashMap<String, CreatureId>` is readable from Tokio I/O tasks but **all insertions and removals are owned exclusively by the game thread**. I/O tasks only call `player_by_name.get(name)` — never `insert` or `remove`. This eliminates the login/logout race on the name slot.

```rust
// Game thread only:
world.player_by_name.insert(name, creature_id);  // on login
world.player_by_name.remove(&name);               // on logout

// I/O tasks (read-only):
let id = world.player_by_name.get(&name).copied();
```

### Tick Budget Watchdog

The game loop wraps each tick body in a timing check:

```rust
let tick_start = Instant::now();
world.tick();
let elapsed = tick_start.elapsed();

if elapsed > Duration::from_millis(45) {
    tracing::warn!(
        elapsed_ms = elapsed.as_millis(),
        "tick budget exceeded — investigate subsystem timing"
    );
}
if elapsed > Duration::from_millis(50) {
    stability_manager.record_error(ErrorCategory::TickOverrun);
}
```

The `StabilityManager` tracks tick overrun counts. If overruns exceed the configured threshold, the recovery handler fires (e.g. temporarily disable non-critical Lua events).

### PropStream Golden Blob Validation

Before Phase 2 implementation, 1,000+ real item and condition blobs are extracted from the live Australis MariaDB:

```sql
SELECT items FROM player_items LIMIT 500;
SELECT conditions FROM players WHERE conditions != '' LIMIT 500;
```

These are committed as binary test fixtures in `tfs-rust-common/tests/fixtures/blobs/`. The PropStream round-trip property test (Property 3) runs against both synthetic data and all golden fixtures, ensuring byte-level compatibility with production data before any player save/load code is written.

Binary serialization for item attributes and condition data stored as DB blobs. The format is byte-compatible with TFS 1.4.2.

```rust
pub struct PropWriteStream {
    buf: Vec<u8>,
}

impl PropWriteStream {
    pub fn write_u8(&mut self, v: u8);
    pub fn write_u16(&mut self, v: u16);
    pub fn write_u32(&mut self, v: u32);
    pub fn write_u64(&mut self, v: u64);
    pub fn write_string(&mut self, s: &str);
    pub fn finish(self) -> Vec<u8>;
}

pub struct PropStream<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> PropStream<'a> {
    pub fn read_u8(&mut self) -> Result<u8>;
    pub fn read_u16(&mut self) -> Result<u16>;
    pub fn read_u32(&mut self) -> Result<u32>;
    pub fn read_u64(&mut self) -> Result<u64>;
    pub fn read_string(&mut self) -> Result<String>;
}
```

### Lua Scripting Bridge

The `LuaState` initializes a LuaJIT runtime and registers all 30+ Lua classes as `mlua::UserData` metatables before loading any scripts. Each Lua object holds a typed ID (`CreatureId`, `ItemId`, `Position`) rather than a raw pointer. Method calls resolve the ID against the `GameWorld` on each invocation.

Script loading order (matching TFS):
1. `data/lib/` — utility libraries
2. `data/events/` — event handler registrations
3. `data/scripts/` — revscript auto-loaded files
4. `data/monster/` — monster scripts
5. `data/npc/` — NPC scripts

The `EventCallback` system stores `Vec<LuaFunction>` per event type. Dispatch iterates the vector, calling each function. If any returns `false`, the chain stops.

### Event System

```rust
pub struct EventDispatcher {
    pub creature_events: HashMap<String, Vec<CreatureEventDef>>,
    pub global_events: HashMap<String, GlobalEventDef>,
    pub move_events: MoveEventRegistry,
    pub actions: ActionRegistry,
    pub talk_actions: TalkActionRegistry,
}
```

Event dispatch is synchronous within the game tick. Lua callbacks are invoked via `lua.call_function(fn, args)` wrapped in error handling.


### Phased Implementation Roadmap

| Phase | Focus | Key Milestone |
|-------|-------|---------------|
| 0 | Workspace scaffold, tfs-rust-common types, CI | `cargo build` green |
| 1 | tfs-rust-net: TCP server, NetworkMessage, XTEA, RSA | Client connects, login handshake works |
| 2 | tfs-rust-db: sqlx pool, player_data, migrations | Player loads/saves from DB |
| 3 | tfs-rust-content: OTBM, OTB, XML loaders | Map and item DB load |
| 4 | tfs-rust-core: GameWorld, Map, Tile, QTree, Position | World ticks, spectator queries work |
| 5 | tfs-rust-core: Creature system (Player, Monster, Npc, Party) | Creatures move and interact |
| 6 | tfs-rust-core: Combat, Conditions, Weapons, Spells | Damage formulas match TFS |
| 7 | tfs-rust-net: ProtocolGame full packet set | Client plays the game |
| 8 | tfs-rust-lua: All 30+ metatables, script loader | All existing scripts load |
| 9 | tfs-rust-core: Event system (all event types) | Login/death/loot/XP events fire |
| 10 | Remaining systems: Chat, Guild, House, Market, VIP, Quests | Feature-complete server |
| 11 | Custom Australis systems: autoloot, dungeon, tasks, rarity | All custom features working |
| 12 | Testing, hardening, load testing, benchmarks | Production-ready |

---

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: XTEA Round Trip

*For any* 8-byte-aligned byte sequence and any 128-bit XTEA key, decrypting the result of encrypting the sequence with that key must produce the original sequence.

**Validates: Requirements 20.1, 4.3**

---

### Property 2: NetworkMessage Round Trip

*For any* sequence of typed values (u8, u16, u32, u64, String, Position) written to a `NetworkMessage` via `write_*` methods, reading them back via the corresponding `read_*` methods must return values equal to the originals.

**Validates: Requirements 20.2, 4.5**

---

### Property 3: PropStream / PropWriteStream Round Trip

*For any* `ItemAttributes` struct, serializing it with `PropWriteStream` and then deserializing the resulting bytes with `PropStream` must produce an `ItemAttributes` equal to the original.

**Validates: Requirements 20.3, 5.9, 10.5**

---

### Property 4: Crate Dependency Invariant

*For any* TFS Rust crate, its declared dependencies must be a subset of the allowed set defined by the architecture (e.g., `tfs-rust-common` has no tfs-rust-* deps; `tfs-rust-net` depends only on `tfs-rust-common`).

**Validates: Requirements 1.2, 1.3, 1.4, 1.5, 1.6, 1.7**

---

### Property 5: Generational Arena Invalidation

*For any* `CreatureId` that has been removed from the `GameWorld`, all subsequent lookups of that ID in the `SlotMap` must return `None`.

**Validates: Requirements 7.1, 7.2, 7.4**

---

### Property 6: Spectator Query Completeness

*For any* set of creatures placed at known positions in the map, `get_spectators(pos, range)` must return exactly the set of `CreatureId` values whose positions fall within the specified range — no more, no fewer.

**Validates: Requirements 7.5**

---

### Property 7: WildcardTree Prefix Correctness

*For any* set of player names inserted into the `WildcardTree` and any query prefix, the result must contain exactly those names that start with the prefix.

**Validates: Requirements 7.7**

---

### Property 8: Combat Damage Formula Equivalence

*For any* valid attacker/target/params combination within the defined input ranges, the Rust `combat::execute` damage output must equal the output of the reference TFS 1.4.2 C++ formula for the same inputs.

**Validates: Requirements 20.4, 9.1, 9.6**

---

### Property 9: A* Path Length Equivalence

*For any* valid start and end position pair on the loaded map, the path length returned by `map.pathfind(from, to, params)` must equal the path length returned by the TFS C++ `AStarNodes` implementation for the same inputs.

**Validates: Requirements 20.5, 11.1**

---

### Property 10: Condition Merge Idempotence

*For any* creature that already has an `ActiveCondition` of a given type and id, applying the same condition again must produce a merged condition whose state is equivalent to what TFS `addCondition` would produce — not a duplicate entry.

**Validates: Requirements 9.4**

---

### Property 11: MatrixArea Transform Correctness

*For any* `MatrixArea`, applying `flip` twice must return the original matrix; applying `rotate90` four times must return the original matrix.

**Validates: Requirements 9.5**

---

### Property 12: zlib Compression Round Trip

*For any* byte sequence, compressing with `flate2` and then decompressing must produce the original sequence.

**Validates: Requirements 4.8**

---

### Property 13: ConfigManager Typed Accessor Round Trip

*For any* configuration key present in a `config.lua` file with a known type, reading it via the corresponding typed accessor must return the value that was written in the file.

**Validates: Requirements 3.2**

---

### Property 14: Missing Config Key Returns Error

*For any* required configuration key that is absent from `config.lua`, `ConfigManager::load` must return an `Err` whose message contains the missing key name.

**Validates: Requirements 3.3**

---

### Property 15: Event Callback Chain Order and Stop

*For any* event type with multiple registered Lua callbacks, the callbacks must be invoked in registration order, and if any callback returns `false`, no subsequent callbacks in the chain must be invoked.

**Validates: Requirements 13.6**

---

### Property 16: Lua Error Isolation

*For any* Lua script that raises an unhandled error during execution, the `LuaState` must catch the error and the game tick must continue processing without panicking or terminating.

**Validates: Requirements 12.6, 22.1**

---

### Property 17: No String-Concatenated SQL

*For any* source file in the `tfs-rust-db` crate, there must be no string concatenation patterns that construct SQL query strings at runtime (i.e., no `format!("SELECT ... WHERE name = {}", name)` patterns in query-executing code).

**Validates: Requirements 5.2**

---

### Property 18: Muted Player Message Rejection

*For any* player with an active mute state, any attempt to send a chat message must be rejected and the mute state must remain unchanged.

**Validates: Requirements 14.4**

---

### Property 19: Offline Training Skill Calculation

*For any* offline training duration and configured skill, the number of skill tries applied on wake must equal `floor(duration_seconds / ticks_per_try)` clamped to the configured maximum.

**Validates: Requirements 18.2**

---

### Property 20: Line-of-Sight Symmetry

*For any* two positions A and B, `map.is_sight_clear(A, B)` must return the same value as `map.is_sight_clear(B, A)`.

**Validates: Requirements 11.2**


---

## Error Handling

### Error Taxonomy

| Category | Type | Handling |
|----------|------|----------|
| Startup failure (missing config, DB unreachable, malformed map) | Fatal | Log error, halt process |
| Content load warning (malformed XML field) | Non-fatal | Log warning, skip entry, continue |
| Network I/O error | Per-connection | Close connection, clean up player session |
| Database transient error | Retryable | Exponential backoff × 3, then log and drop operation |
| Lua script error | Isolated | Catch via mlua error handling, log with context, continue tick |
| Game logic error (invalid state) | Defensive | Log with `tracing::error!`, return `ReturnValue::NotPossible` |

### Error Types

```rust
// tfs-rust-common/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum TfsRustError {
    #[error("config error: {0}")]
    Config(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("network error: {0}")]
    Network(#[from] std::io::Error),
    #[error("content error in {file}: {message}")]
    Content { file: String, message: String },
    #[error("lua error: {0}")]
    Lua(#[from] mlua::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
}
```

### Graceful Shutdown

On SIGINT/SIGTERM:
1. Stop accepting new connections
2. Send `GameCommand::Shutdown` to game thread
3. Game thread saves all online players via `player_data::save_player` (async, awaited)
4. Close all connections (send disconnect packet)
5. Flush DB pool
6. Exit within 30 seconds (enforced by a `tokio::time::timeout`)

### Stability Manager

```rust
pub struct StabilityManager {
    pub error_counts: DashMap<ErrorCategory, AtomicU64>,
    pub recovery_handlers: HashMap<ErrorCategory, Box<dyn Fn() + Send + Sync>>,
    pub thresholds: HashMap<ErrorCategory, u64>,
}
```

When a category's count exceeds its threshold, the registered recovery handler is invoked (e.g., reload scripts, reconnect DB pool).

---

## Testing Strategy

### Dual Testing Approach

Both unit tests and property-based tests are required. They are complementary:
- Unit tests catch concrete bugs with specific inputs and verify integration points
- Property tests verify universal correctness across the full input space

### Property-Based Testing

**Library:** `proptest` (Rust). Each property test runs a minimum of 100 iterations.

Each property test is tagged with a comment referencing the design property:

```rust
// Feature: tfs-rust-rust-engine, Property 1: XTEA Round Trip
proptest! {
    #[test]
    fn xtea_round_trip(data in prop::collection::vec(any::<u8>(), 0..=256).prop_map(|mut v| {
        v.resize((v.len() + 7) / 8 * 8, 0); v
    }), key in any::<[u32; 4]>()) {
        let mut encrypted = data.clone();
        xtea::encrypt(&mut encrypted, &key);
        xtea::decrypt(&mut encrypted, &key);
        prop_assert_eq!(encrypted, data);
    }
}
```

**Property test coverage:**

| Property | Test Location | Library |
|----------|--------------|---------|
| 1: XTEA Round Trip | `tfs-rust-net/tests/xtea.rs` | proptest |
| 2: NetworkMessage Round Trip | `tfs-rust-net/tests/message.rs` | proptest |
| 3: PropStream Round Trip | `tfs-rust-common/tests/propstream.rs` | proptest |
| 4: Crate Dependency Invariant | `tests/workspace.rs` | proptest + cargo_metadata |
| 5: Generational Arena Invalidation | `tfs-rust-core/tests/arena.rs` | proptest |
| 6: Spectator Query Completeness | `tfs-rust-core/tests/map.rs` | proptest |
| 7: WildcardTree Prefix Correctness | `tfs-rust-core/tests/wildcard.rs` | proptest |
| 8: Combat Damage Formula Equivalence | `tfs-rust-core/tests/combat.rs` | proptest |
| 9: A* Path Length Equivalence | `tfs-rust-core/tests/pathfinding.rs` | proptest |
| 10: Condition Merge Idempotence | `tfs-rust-core/tests/conditions.rs` | proptest |
| 11: MatrixArea Transform Correctness | `tfs-rust-core/tests/matrix.rs` | proptest |
| 12: zlib Compression Round Trip | `tfs-rust-net/tests/compression.rs` | proptest |
| 13: ConfigManager Round Trip | `tfs-rust-core/tests/config.rs` | proptest |
| 14: Missing Config Key Error | `tfs-rust-core/tests/config.rs` | proptest |
| 15: Event Callback Chain Order | `tfs-rust-core/tests/events.rs` | proptest |
| 16: Lua Error Isolation | `tfs-rust-lua/tests/error_isolation.rs` | proptest |
| 17: No String-Concatenated SQL | `tfs-rust-db/tests/sql_safety.rs` | static analysis |
| 18: Muted Player Rejection | `tfs-rust-core/tests/chat.rs` | proptest |
| 19: Offline Training Calculation | `tfs-rust-core/tests/bed.rs` | proptest |
| 20: Line-of-Sight Symmetry | `tfs-rust-core/tests/map.rs` | proptest |

### Unit Tests

Unit tests focus on:
- Specific protocol packet encoding/decoding examples (one test per opcode)
- RSA decryption with a known key/ciphertext pair
- Content loader parsing of known-good and known-bad files
- Database migration runner with an in-memory SQLite DB (for schema logic)
- Script loader loading all 994 NPC scripts and 604 monster scripts without error
- Full login → play → logout integration cycle

### Integration Tests

- `tests/protocol_compat.rs` — compare Rust packet bytes against captured TFS output for all 60+ outgoing packet types
- `tests/lua_compat.rs` — load every script in `data/npc/` and `data/monster/`, assert no errors
- `tests/db_compat.rs` — write a player record with TFS Rust, read it back with TFS schema queries, assert equivalence

