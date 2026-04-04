# Implementation Plan: TFS Rust Rust Engine

## Overview

Ground-up Rust rewrite of the Australis TFS 1.4.2 C++ game server as a Cargo workspace of six crates. Implementation follows a strict dependency-ordered 13-phase roadmap: `tfs-rust-common` first, then `tfs-rust-net`/`tfs-rust-db`/`tfs-rust-content` in parallel, then `tfs-rust-core`, then `tfs-rust-lua`, then integration and hardening. Each phase ends with `cargo check && cargo clippy && cargo fmt --check` passing clean.

## Tasks

- [x] 0. Phase 0 — Workspace scaffold and tfs-rust-common foundation
  - [x] 0.1 Initialize Cargo workspace with all six member crates
    - Create `tfs-rust/Cargo.toml` workspace root listing all six members
    - Scaffold `crates/tfs-rust-common`, `crates/tfs-rust-content`, `crates/tfs-rust-db`, `crates/tfs-rust-net`, `crates/tfs-rust-lua`, `crates/tfs-rust-core` with minimal `lib.rs` stubs
    - Create `src/main.rs` entry point that calls `tfs_rust_core::run()`
    - Add workspace-level `[dependencies]` for shared crates (tokio, tracing, thiserror, anyhow)
    - _Requirements: 1.1, 1.9_

  - [x] 0.2 Implement `tfs-rust-common`: Position, Direction, and core enums
    - Define `Position { x: u16, y: u16, z: u8 }` with `distance_to`, `get_direction_to`, `offset`, and range-check methods matching TFS semantics
    - Define all game enums: `Direction`, `CombatType`, `ConditionType`, `SkullType`, `ZoneType`, `ReturnValue`, `ItemGroup`, `WeaponType`, `Skill`, `PlayerSex`, `WorldType`, `SpeakType`, `MagicEffect`, `ShootEffect`
    - _Requirements: 1.2, 7.8_

  - [x] 0.3 Implement `tfs-rust-common`: PropStream / PropWriteStream
    - Implement `PropWriteStream` with `write_u8/u16/u32/u64/string` and `finish() -> Vec<u8>`
    - Implement `PropStream<'a>` with `read_u8/u16/u32/u64/string` returning `Result<T, TfsRustError>`
    - Byte layout must be compatible with TFS 1.4.2 blob format
    - _Requirements: 1.2, 5.9, 10.5_

  - [x] 0.4 Implement `tfs-rust-common`: TfsRustError unified error type
    - Define `TfsRustError` via `thiserror` with variants: `Config`, `Database`, `Network`, `Content`, `Lua`, `Protocol`
    - _Requirements: 1.2_

  - [x]* 0.5 Write property test for PropStream round trip (Property 3)
    - **Property 3: PropStream / PropWriteStream Round Trip**
    - **Validates: Requirements 20.3, 5.9, 10.5**
    - Test file: `tfs-rust-common/tests/propstream.rs`

  - [x] 0.6 Set up CI: cargo check, clippy, fmt
    - Add `.github/workflows/ci.yml` running `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt --check`, and `cargo test` on every push
    - _Requirements: 1.9_

  - [x] 0.7 Extract golden blob test fixtures from live Australis MariaDB
    - Run SQL to extract 1,000+ item blobs and 500+ condition blobs from the live database: `SELECT items FROM player_items LIMIT 500` and `SELECT conditions FROM players WHERE conditions != ''`
    - Commit binary fixtures to `tfs-rust-common/tests/fixtures/blobs/`
    - Write a fixture-driven test asserting `PropStream` can deserialize all blobs without error (run this before Phase 2)
    - _Requirements: 23.1, 23.2, 23.4_

- [x] 1. Phase 1 — tfs-rust-net: TCP server, NetworkMessage, XTEA, RSA, protocols
  - [x] 1.1 Implement `NetworkMessage` byte buffer
    - Implement `NetworkMessage { buf: BytesMut, read_pos: usize }` with typed `read_*` / `write_*` methods for `u8`, `u16`, `u32`, `u64`, `String` (length-prefixed), and `Position` in little-endian byte order
    - _Requirements: 4.5_

  - [x]* 1.2 Write property test for NetworkMessage round trip (Property 2)
    - **Property 2: NetworkMessage Round Trip**
    - **Validates: Requirements 20.2, 4.5**
    - Test file: `tfs-rust-net/tests/message.rs`

  - [x] 1.3 Implement XTEA encrypt/decrypt
    - Implement `xtea::encrypt(data: &mut [u8], key: &[u32; 4])` and `xtea::decrypt` as 64-round Feistel cipher on 8-byte blocks, output byte-for-byte identical to TFS C++ implementation
    - _Requirements: 4.3_

  - [x]* 1.4 Write property test for XTEA round trip (Property 1)
    - **Property 1: XTEA Round Trip**
    - **Validates: Requirements 20.1, 4.3**
    - Test file: `tfs-rust-net/tests/xtea.rs`

  - [x] 1.5 Implement RSA login block decryption
    - Implement `rsa::decrypt(block: &[u8; 128], private_key: &RsaPrivateKey) -> Result<Vec<u8>>` using the `rsa` crate (FFI boundary — only permitted `unsafe` site)
    - _Requirements: 4.4_

  - [x] 1.6 Implement `ConnectionState` state machine and TCP server
    - Define `ConnectionState { Handshake, Login(ProtocolLogin), Game(ProtocolGame), Status(ProtocolStatus), Closed }`
    - Implement `Server` as a Tokio TCP listener that spawns one `Connection` task per accepted socket
    - Wire `XteaState { key: [u32; 4], enabled: bool }` into each connection
    - _Requirements: 4.1, 4.2, 4.10_

  - [x] 1.6b Add `PendingLogin` state to `ConnectionState`
    - Add `PendingLogin { conn_id: ConnId, char_name: String }` variant — used while awaiting the DB oneshot result across tick boundaries
    - Packets arriving in `PendingLogin` state are queued or dropped by type (movement/attack dropped, chat queued)
    - If the connection closes in `PendingLogin`, the game thread discards the oneshot result when it arrives
    - This state is required before Phase 5 login flow can be implemented correctly
    - _Requirements: 4.10_

  - [x] 1.7 Implement `ProtocolLogin`: RSA decrypt, character list response
    - Parse the first login message: RSA-decrypt the 128-byte block, extract account/password, query DB for character list, send character list or error packet
    - _Requirements: 4.1_

  - [x] 1.8 Implement `ProtocolStatus`: XML status response
    - Respond to status port queries with a valid XML status document
    - _Requirements: 4.9_

  - [x] 1.9 Define `GameCommand` enum covering all 50+ client packet types
    - Define all variants: `PlayerMove`, `PlayerSay`, `PlayerUseItem`, `PlayerAttack`, `PlayerLogin`, `PlayerLogout`, `ExtendedOpcode`, and all remaining TFS client opcodes
    - _Requirements: 4.6_

  - [x] 1.10 Implement zlib compression/decompression in NetworkMessage
    - Integrate `flate2` compress/decompress into the outgoing/incoming pipeline, gated by config flag
    - _Requirements: 4.8_

  - [x]* 1.11 Write property test for zlib compression round trip (Property 12)
    - **Property 12: zlib Compression Round Trip**
    - **Validates: Requirements 4.8**
    - Test file: `tfs-rust-net/tests/compression.rs`

  - [x] 1.12 Checkpoint — cargo check/clippy/fmt pass on tfs-rust-net
    - Ensure all tests pass, ask the user if questions arise.

  - [x] 1.13 Manual checkpoint — attempt real OTClient connection _(done: TCP accept verified with `tcp_smoke` + OTClient (protocol **10.98**) → `127.0.0.1:7171`; see `docs/phase1-otclient-checkpoint.md`)_
    - Start the server and attempt a connection from OTClient configured for protocol **10.98**
    - Verify TCP reaches the server (`accepted TCP connection` in the smoke example); full app login (RSA decrypt + character list) is deferred until the main server wires `ProtocolLogin` (Phase 5 path)
    - This catches framing and transport issues that round-trip unit tests will not find
    - Document any framing issues found before proceeding to Phase 2


- [x] 2. Phase 2 — tfs-rust-db: sqlx pool, player data, migrations, market
  - [x] 2.1 Implement `DbPool` with sqlx MariaDB connection pool
    - Wrap `sqlx::MySqlPool` with configurable min/max connections; implement retry policy: up to 3 retries with exponential backoff (100ms, 200ms, 400ms) on transient errors
    - _Requirements: 5.1, 5.8_

  - [x] 2.2 Implement database migrations runner
    - Check schema version at startup; apply pending migrations before accepting connections
    - _Requirements: 5.5_

  - [x] 2.2b Run `cargo sqlx prepare` and commit `.sqlx/` offline query cache
    - Run `DATABASE_URL=... cargo sqlx prepare --workspace -- --workspace` against a MariaDB/MySQL with the TFS schema (or any DB for the anchor `query!`); the second `--workspace` is forwarded to `cargo check` so all crates compile
    - Commit the `.sqlx/` directory so `cargo check` and CI pass without a live DB connection
    - Without this step, any use of `query!` or `query_as!` macros will fail in CI with "no DATABASE_URL set"
    - _Requirements: 1.9_

  - [x] 2.3 Implement `player_data`: load_player and save_player
    - Load/save complete player record (stats, inventory, skills, storage, conditions, spells, vip list, guild, outfits, mounts) using TFS 1.4.2 schema
    - Use `PropWriteStream`/`PropStream` for item attribute and condition blobs
    - All queries must be sqlx prepared statements — no string-concatenated SQL
    - _Requirements: 5.2, 5.3, 5.4, 5.9_

  - [x] 2.4 Implement `player_data`: load_items and save_items
    - Serialize/deserialize item trees (inventory, depot, inbox) to/from DB blobs using `PropWriteStream`/`PropStream`
    - _Requirements: 5.3, 5.4, 5.9_

  - [x] 2.5 Implement `market` module: create, accept, cancel, browse offers
    - Implement all market operations using async prepared statements; persist offers and restore on restart
    - _Requirements: 5.6, 17.1, 17.2, 17.3, 17.4, 17.5_

  - [x] 2.6 Implement house item serialization via MapSerialize
    - Load/save house item contents and ownership using `PropStream`/`PropWriteStream` for blob serialization
    - _Requirements: 5.7_

  - [x]* 2.7 Write static analysis test for no string-concatenated SQL (Property 17)
    - **Property 17: No String-Concatenated SQL**
    - **Validates: Requirements 5.2**
    - Test file: `tfs-rust-db/tests/sql_safety.rs` — scan source AST/text for `format!` patterns in query-executing code

  - [x] 2.8 Checkpoint — cargo check/clippy/fmt pass on tfs-rust-db
    - Ensure all tests pass, ask the user if questions arise.

- [x] 3. Phase 3 — tfs-rust-content: OTBM, OTB, XML loaders
  - [x] 3.1 Implement `OtbLoader`: parse OTB binary item database
    - Populate `ItemDatabase` with all item type definitions from the OTB binary format
    - _Requirements: 6.2_

  - [x] 3.2 Implement `ItemDatabase`: merge OTB + items.xml
    - Parse `items.xml` to load extended attributes not in OTB; merge into `HashMap<u16, ItemType>`
    - Log descriptive error with file path and offset on malformed entries; halt startup on fatal errors
    - _Requirements: 6.3, 6.8_

  - [x] 3.3 Implement `OtbmLoader`: parse OTBM binary map format
    - Load all tile areas, tile items, spawns, houses, towns, and waypoints into `MapData`
    - Expose waypoints via `world.get_waypoint_by_name(name)` and towns via Town registry
    - _Requirements: 6.1, 6.6, 6.7, 6.8_

  - [x] 3.4 Implement `MonsterDatabase`: parse monsters/ XML files
    - Populate `MonsterType` registry with all monster definitions including loot tables and spell lists
    - _Requirements: 6.4, 6.8_

  - [x] 3.5 Implement `VocationDatabase`, `OutfitDatabase`, `MountDatabase`, `GroupDatabase`
    - Parse `vocations.xml`, `outfits.xml`, `mounts.xml`, `groups.xml` into their respective registries
    - _Requirements: 6.5_

  - [x] 3.6 Wire concurrent content loading via tokio::join!
    - Load all independent content files concurrently; total load time must be less than sequential
    - _Requirements: 6.9_

  - [x] 3.7 Checkpoint — cargo check/clippy/fmt pass on tfs-rust-content
    - Ensure all tests pass, ask the user if questions arise.


- [x] 4. Phase 4 — tfs-rust-core: GameWorld, Map, Tile, QTree, pathfinding, config
  - [x] 4.0 Define `EventDispatcher` trait and `LuaCommand` buffer in tfs-rust-core
    - Define `pub trait EventDispatcher: Send + 'static` with methods for all event types (onLogin, onDeath, onKill, onThink, etc.)
    - Define `pub enum LuaCommand` covering all game-state mutations that Lua scripts may request (teleport, set health, send message, etc.)
    - `GameWorld` stores `Box<dyn EventDispatcher>` injected at construction — it does NOT import tfs-rust-lua
    - This resolves the circular dependency: core → lua → core is a Cargo hard error; the trait breaks the cycle
    - Remove `tfs-rust-lua` from `tfs-rust-core/Cargo.toml`
    - _Requirements: 1.7_

  - [x] 4.1 Implement `ConfigManager`: load config.lua via mlua
    - Execute `config.lua` through mlua at startup; expose `get_string`, `get_number::<T>`, `get_bool` typed accessors
    - Return descriptive `Err` identifying missing key name for any absent required key; halt startup
    - _Requirements: 3.1, 3.2, 3.3, 3.4_

  - [ ]* 4.2 Write property test for ConfigManager round trip (Property 13)
    - **Property 13: ConfigManager Typed Accessor Round Trip**
    - **Validates: Requirements 3.2**
    - Test file: `tfs-rust-core/tests/config.rs`

  - [ ]* 4.3 Write property test for missing config key returns error (Property 14)
    - **Property 14: Missing Config Key Returns Error**
    - **Validates: Requirements 3.3**
    - Test file: `tfs-rust-core/tests/config.rs`

  - [x] 4.4 Implement `GameWorld` struct and slotmap entity arenas
    - Define `GameWorld` owning `SlotMap<CreatureId, CreatureKind>`, `SlotMap<ItemId, Item>`, `Map`, `Scheduler`, `Box<dyn EventDispatcher>`, `Config`, `DbPool`
    - NOTE: `GameWorld` does NOT own `LuaState` directly — Lua is injected via the `EventDispatcher` trait
    - Add `DashMap<String, CreatureId>` for player-by-name and `DashMap<u32, CreatureId>` for player-by-guid concurrent lookups
    - _Requirements: 7.1, 7.2, 7.3, 21.3_

  - [x]* 4.5 Write property test for generational arena invalidation (Property 5)
    - **Property 5: Generational Arena Invalidation**
    - **Validates: Requirements 7.1, 7.2, 7.4**
    - Test file: `tfs-rust-core/tests/arena.rs`

  - [x] 4.6 Implement `Tile` struct with query_add, query_remove, add_thing, remove_thing
    - Implement `Tile { position, ground, items, creatures, flags, zone, house_id }` with all TFS accessor methods
    - Implement `HouseTile` variant extending Tile with house reference
    - _Requirements: 11.5, 11.6_

  - [x] 4.7 Implement `QTreeNode` quadtree spatial index with sector spectator cache
    - Implement `QTreeNode { Branch, Leaf }` covering 32×32 tile sectors
    - Add `cached_spectators: Option<Vec<CreatureId>>` to each leaf node; invalidate cache only on creature enter/leave, not on every query
    - Implement `get_spectators(pos, range)` returning all `CreatureId` values within range — O(k log n), with cache hits avoiding full traversal
    - _Requirements: 7.5, 21.7_

  - [ ]* 4.8 Write property test for spectator query completeness (Property 6)
    - **Property 6: Spectator Query Completeness**
    - **Validates: Requirements 7.5**
    - Test file: `tfs-rust-core/tests/map_spectators.rs` (not yet added)

  - [x] 4.9 Implement `Map` struct with get_tile, get_tile_mut, can_throw_to, is_sight_clear
    - Implement `Map { root: QTreeNode, width, height, houses, spawns, towns, waypoints }`
    - Implement `get_tile(pos)` / `get_tile_mut(pos)` returning `Option<&Tile>` / `Option<&mut Tile>`
    - Implement `can_throw_to` and `is_sight_clear` line-of-sight checks matching TFS behavior
    - _Requirements: 7.6, 11.2_

  - [x]* 4.10 Write property test for line-of-sight symmetry (Property 20)
    - **Property 20: Line-of-Sight Symmetry**
    - **Validates: Requirements 11.2**
    - Test file: `tfs-rust-core/tests/map_los.rs`

  - [x] 4.11 Implement A* pathfinding
    - Implement `pathfind(map, from, to, params) -> Option<Vec<Direction>>` using `BinaryHeap<AStarNode>` open set and closed set, matching TFS `AStarNodes` cost function (walkability, creature blocking, diagonal cost)
    - _Requirements: 11.1_

  - [ ]* 4.12 Write property test for A* path length equivalence (Property 9)
    - **Property 9: A* Path Length Equivalence**
    - **Validates: Requirements 20.5, 11.1**
    - Test file: `tfs-rust-core/tests/pathfinding.rs`

  - [x] 4.13 Implement `WildcardTree` trie for player name prefix lookup
    - Implement trie supporting `insert(name)`, `remove(name)`, and `get_by_prefix(prefix) -> Vec<String>`
    - _Requirements: 7.7_

  - [x]* 4.14 Write property test for WildcardTree prefix correctness (Property 7)
    - **Property 7: WildcardTree Prefix Correctness**
    - **Validates: Requirements 7.7**
    - Test file: `tfs-rust-core/tests/wildcard.rs`

  - [x] 4.15 Implement `DecayManager` and item decay tick
    - Track all decaying items; on each game tick advance timers and transform or remove elapsed items
    - _Requirements: 10.4_

  - [x] 4.16 Implement `SpawnManager`: load spawns, create initial monsters, respawn on death
    - Load spawn definitions from OTBM data; create initial monsters at startup; respawn after configured interval
    - _Requirements: 11.4_

  - [x] 4.17 Implement `HouseManager`: owner, access list, bed tracking, tile ownership
    - Implement owner assignment, guest/subowner/door access lists, player kick, bed tracking; persist to DB
    - _Requirements: 11.3_

  - [x] 4.18 Implement game loop: tokio::time::interval(50ms) driving GameWorld::tick()
    - Implement tick order: drain mpsc channel → process_creature_walk → process_creature_think → process_condition_ticks → process_decay → process_light_cycle → lua_gc_step (every 5 ticks) → flush_output_buffers
    - Implement `lua_gc_step()`: call `lua.gc(LuaGCMode::Step, 200)` every 5 ticks to spread LuaJIT GC across multiple ticks, preventing stop-the-world pauses from stale `PlayerRef`/`CreatureRef` wrapper objects
    - Implement tick budget watchdog: record `Instant::now()` before tick; emit `tracing::warn!` if elapsed > 45ms; call `stability_manager.record_error(ErrorCategory::TickOverrun)` if elapsed > 50ms
    - _Requirements: 2.1, 2.2, 21.6, 21.10_

  - [x] 4.19 Implement `OutputMessage` send queue with backpressure
    - Implement `ConnectionSendQueue { queue: VecDeque<OutputMessage>, max_size: usize, bytes_pending: usize }`
    - When `bytes_pending` exceeds limit: drop stale visual-only packets (map chunks, magic effects, creature movement) before critical packets (stats, inventory, text)
    - Split large sends (depot open, full map description) across multiple ticks via `pending_chunks` queue
    - Close connection after 3 consecutive ticks with a full queue
    - _Requirements: 21.8, 21.9_

  - [x] 4.20 Enforce game-thread-only ownership of player name/guid maps
    - Document and enforce via code review: `player_by_name.insert/remove` and `player_by_guid.insert/remove` are called only from the game thread
    - I/O tasks use only `player_by_name.get()` and `player_by_guid.get()` (read-only)
    - Add a lint comment to each insertion/removal site: `// GAME THREAD ONLY`
    - _Requirements: 21.11_

  - [x] 4.21 Implement `Scheduler`: addEvent / stopEvent via tokio::time::sleep_until + channel dispatch
    - Accept duration + closure; spawn Tokio task that sleeps then sends `GameCommand::LuaCallback` to game thread
    - _Requirements: 2.6_

  - [x] 4.22 Implement graceful shutdown on SIGINT/SIGTERM
    - Stop accepting connections → send `GameCommand::Shutdown` → save all online players → close connections → flush DB pool → exit within 30s via `tokio::time::timeout`
    - _Requirements: 2.7, 22.4_

  - [x] 4.23 Checkpoint — cargo check/clippy/fmt pass on tfs-rust-core Phase 4
    - Ensure all tests pass, ask the user if questions arise.



- [x] 5. Phase 5 — Creature system: Player, Monster, Npc, Party, Guild, login flow
  - [x] 5.1 Implement `CreatureBase` struct
    - Define all shared fields: id, name, position, direction, health, max_health, outfit, speed, base_speed, skull, conditions, walk_queue, follow_target, attack_target, master, damage_map
    - _Requirements: 8.1_

  - [x] 5.2 Implement `Player` struct with sub-structs
    - Decompose into `PlayerInventory`, `PlayerSkills`, `PlayerEconomy`, `PlayerSocial` sub-structs
    - Implement level-up logic: recalculate max_health, max_mana, capacity per vocation formula on experience gain
    - _Requirements: 8.2, 8.3_

  - [x] 5.3 Implement `Monster` struct and AI loop
    - Implement full TFS AI entirely in native Rust: target selection, flee behavior, friend/target list management, idle detection, return-to-spawn walk, per-think sub-routines (target, yell, defense)
    - Gate Lua `onThink` FFI call: only invoke `creature_events.execute_think(lua, creature, interval)` when `creature.base.registered_events` contains `"onThink"` — do NOT cross the FFI boundary for monsters without this registration
    - _Requirements: 8.4, 8.5_

  - [x] 5.4 Implement `Npc` struct and event model
    - Implement `on_appear`, `on_disappear`, `on_say`, `on_buy`, `on_sell`, `on_check_item`, `on_close_channel` dispatching to NPC Lua script via `NpcEventsHandler`
    - _Requirements: 8.5_

  - [x] 5.5 Implement `Party` struct: shared XP, leadership transfer, invitations
    - Implement shared experience distribution using Australis custom formula; party leadership transfer; invitation management; shared experience toggle
    - _Requirements: 8.6_

  - [x] 5.6 Implement `Guild` struct: load from DB, MOTD, war state
    - Load guild data (name, motd, ranks, members) at player login; send MOTD on login; expose `is_in_war(player_a, player_b)`
    - _Requirements: 15.1, 15.2, 15.3_

  - [x] 5.7 Implement summon model
    - Implement `is_summon()` returning true when `base.master` is set; wire master/summon relationship into creature removal
    - _Requirements: 8.8_

  - [x] 5.8 Implement player login flow: DB load → place in world → send initial packets
    - On `GameCommand::PlayerLogin`: load player from DB, place creature in world, add to name/guid maps, send login packets to client
    - _Requirements: 5.3_

  - [x] 5.9 Implement creature death sequence
    - Drop corpse, drop loot, attribute XP to killers by damage ratio, fire `onKill` and `onDeath` creature events, schedule corpse decay
    - _Requirements: 8.7_

  - [x] 5.10 Checkpoint — cargo check/clippy/fmt pass on creature system
    - Ensure all tests pass, ask the user if questions arise.

- [x] 6. Phase 6 — Combat system: damage formulas, conditions, weapons, spells
  - [x] 6.1 Implement `CombatParams` struct and `combat::execute` entry point
    - Implement `execute(world, attacker, params)` dispatching to health, mana, condition, or dispel sub-functions based on `combat_type`
    - Implement `can_attack(world, attacker, target)` enforcing PvP rules: protection zone, skull mode, secure mode, world type
    - _Requirements: 9.1, 9.2_

  - [x] 6.2 Implement `ConditionData` enum with all variants
    - Define `Damage`, `Speed`, `Outfit`, `Light`, `Regeneration`, `Soul`, `Attributes`, `SpellCooldown`, `SpellGroupCooldown`, `Generic` variants
    - Implement `addCondition` merge logic: when same type+id already exists, merge rather than duplicate
    - _Requirements: 9.3, 9.4_

  - [x]* 6.3 Write property test for condition merge idempotence (Property 10)
    - **Property 10: Condition Merge Idempotence**
    - **Validates: Requirements 9.4**
    - Test file: `tfs-rust-core/tests/conditions.rs`

  - [x] 6.4 Implement `MatrixArea` with flip, mirror, rotate90 transforms
    - Implement `MatrixArea { data: Vec<Vec<bool>>, center_x, center_y }` with `flip`, `mirror`, `rotate90` producing results identical to TFS C++ implementation
    - _Requirements: 9.5_

  - [x]* 6.5 Write property test for MatrixArea transform correctness (Property 11)
    - **Property 11: MatrixArea Transform Correctness**
    - **Validates: Requirements 9.5**
    - Test file: `tfs-rust-core/tests/matrix.rs`

  - [x] 6.6 Implement melee, distance, and wand weapon damage calculations
    - Implement `Weapon` module with element damage, producing results identical to TFS 1.4.2 formulas
    - _Requirements: 9.6_

  - [x]* 6.7 Write property test for combat damage formula equivalence (Property 8)
    - **Property 8: Combat Damage Formula Equivalence**
    - **Validates: Requirements 20.4, 9.1, 9.6**
    - Test file: `tfs-rust-core/tests/combat.rs`

  - [x] 6.8 Implement `Spell` module: instant spells and rune spells
    - Enforce cooldown, mana cost, soul cost, level requirement, vocation restriction; apply `MatrixArea` effects
    - _Requirements: 9.7, 9.8_

  - [x] 6.9 Checkpoint — cargo check/clippy/fmt pass on combat system
    - Ensure all tests pass, ask the user if questions arise.


- [ ] 7. Phase 7 — Game protocol: all 50+ incoming parsers, all 60+ outgoing builders
  - [ ] 7.1 Implement `ProtocolGame` incoming packet dispatcher (all 50+ opcodes)
    - Parse all 50+ incoming client packet opcodes defined in Tibia **10.98** protocol; dispatch each to the corresponding `GameCommand` variant via mpsc channel
    - _Requirements: 4.2, 4.6_

  - [ ] 7.2 Implement `ProtocolGame` outgoing packet builders (all 60+ opcodes)
    - Implement all 60+ outgoing server packet builders producing byte sequences identical to TFS 1.4.2
    - _Requirements: 4.7_

  - [ ] 7.3 Implement map description packet builder
    - Implement `sendMapDescription(pos, range)` building the full tile/creature/item description packet for the visible area
    - _Requirements: 4.7_

  - [ ] 7.4 Implement OTCv8 extended opcode support with async-safe handler context
    - Dispatch `parseExtendedOpcode` messages to registered Lua `PacketHandler` callbacks
    - Enforce async-only DB access in the extended opcode handler context: expose only `db.asyncQuery` and `db.asyncStoreQuery`; block `db.query` and `db.storeQuery` to prevent game tick stalls
    - Async results return via `GameCommand::LuaAsyncResult` on the next tick
    - _Requirements: 4.11, 4.12_

  - [ ] 7.5 Wire ProtocolGame into GameWorld: connect mpsc channel, flush output buffers each tick
    - Connect `cmd_tx` from each `ProtocolGame` to the game thread's `cmd_rx`; implement `flush_output_buffers()` sending all queued packets at end of each tick
    - _Requirements: 2.4, 4.2_

  - [ ]* 7.6 Write unit tests for all 60+ outgoing packet encodings
    - Compare Rust packet bytes against captured TFS output for all outgoing packet types
    - Test file: `tests/protocol_compat.rs`
    - _Requirements: 20.2_

  - [ ] 7.7 Checkpoint — cargo check/clippy/fmt pass on protocol layer
    - Ensure all tests pass, ask the user if questions arise.

- [ ] 8. Phase 8 — tfs-rust-lua: LuaState, all 30+ metatables, script loader, EventCallback
  - [ ] 8.1 Implement `LuaState` initialization and GameWorldHandle app data
    - Initialize LuaJIT runtime via mlua; store `GameWorldHandle` as app data for method call resolution
    - _Requirements: 12.1_

  - [ ] 8.2 Implement `PlayerRef`, `CreatureRef`, `ItemRef`, `TileRef` UserData wrappers
    - Each wrapper holds a typed ID (`CreatureId`, `ItemId`, `Position`); resolves against `GameWorld` on each method call; returns Lua error if entity no longer exists
    - _Requirements: 12.1, 12.2_

  - [ ] 8.3 Implement all ~500 Lua methods across all 30+ metatables
    - Implement all methods on `Player`, `Creature`, `Monster`, `Npc`, `Item`, `Container`, `Tile`, `Position`, `Combat`, `Condition`, `Party`, `Guild`, `House`, `Channel`, `Game`, `Db`, `ConfigManager`, and all remaining TFS Lua classes
    - Signatures and behavior must be identical to TFS 1.4.2 `luascript.cpp`
    - _Requirements: 12.2_

  - [ ] 8.4 Implement `NpcScriptInterface` NPC-only Lua functions
    - Expose `doNpcSetCreatureFocus`, `openShopWindow`, `closeShopWindow`, `doSellItem`, `doBuyItem`, `doSendCreatureSay` only within NPC script contexts
    - _Requirements: 12.4_

  - [ ] 8.5 Implement Lua `db` table: query, asyncQuery, storeQuery, escapeString, tableExists, lastInsertId
    - Maintain API compatibility with TFS; async variants dispatch to Tokio tasks
    - _Requirements: 12.7_

  - [ ] 8.6 Implement Lua `configManager` table: getString, getNumber, getBoolean
    - _Requirements: 12.8_

  - [ ] 8.7 Implement `addEvent` / `stopEvent` global Lua functions
    - Schedule/cancel delayed Lua callbacks via Tokio scheduler; behavior identical to TFS
    - _Requirements: 12.9_

  - [ ] 8.8 Implement `EventCallback` registry: HashMap<EventType, Vec<LuaFunction>>
    - Dispatch iterates vector in registration order; stop chain if any callback returns `false`
    - _Requirements: 12.5, 13.6_

  - [ ]* 8.9 Write property test for event callback chain order and stop (Property 15)
    - **Property 15: Event Callback Chain Order and Stop**
    - **Validates: Requirements 13.6**
    - Test file: `tfs-rust-core/tests/events.rs`

  - [ ] 8.10 Implement `ScriptLoader`: discover and load scripts in TFS order
    - Load in order: `data/lib/` → `data/events/` → `data/scripts/` → `data/monster/` → `data/npc/`
    - _Requirements: 12.3, 12.10_

  - [ ] 8.11 Implement Lua error isolation: catch errors, log with context, continue tick
    - Wrap every Lua call in mlua error handling; log script name, line number, error message; game tick must continue without panic
    - _Requirements: 12.6, 22.1_

  - [ ]* 8.12 Write property test for Lua error isolation (Property 16)
    - **Property 16: Lua Error Isolation**
    - **Validates: Requirements 12.6, 22.1**
    - Test file: `tfs-rust-lua/tests/error_isolation.rs`

  - [ ] 8.13 Checkpoint — cargo check/clippy/fmt pass on tfs-rust-lua
    - Ensure all tests pass, ask the user if questions arise.

  - [ ] 8.14 Early script validation — run 50–100 representative scripts and fix API gaps
    - Before building out the full ~500 method API surface, load a representative sample: 10 NPC scripts, 10 monster scripts, 10 action scripts, 10 talkaction scripts, 10 creatureevent scripts
    - Fix all missing method errors found — these are the long tail of small API gaps that are far cheaper to fix now than after Phase 9
    - Document any methods that require Phase 9 event system work to implement (defer those, don't stub them silently)
    - _Requirements: 12.10_


- [ ] 9. Phase 9 — Event system: CreatureEvents, GlobalEvents, MoveEvents, Actions, TalkActions
  - [ ] 9.1 Implement `CreatureEvents` system
    - Fire `onLogin`, `onLogout`, `onThink`, `onPrepareDeath`, `onDeath`, `onKill`, `onAdvance`, `onModalWindow`, `onTextEdit`, `onHealthChange`, `onManaChange`, `onExtendedOpcode`, `onMoveItem` to all registered Lua callbacks
    - _Requirements: 13.1_

  - [ ] 9.2 Implement `GlobalEvents` system
    - Fire `onStartup`, `onShutdown`, `onRecord`, `onTime`, `onTimer`, `onPeriodChange` to all registered Lua callbacks
    - _Requirements: 13.2_

  - [ ] 9.3 Implement `MoveEvents` system
    - Fire `onAddItem`, `onRemoveItem`, `onMoveItem`, `onStepIn`, `onStepOut` when items or creatures interact with registered tiles or item types
    - _Requirements: 13.3_

  - [ ] 9.4 Implement `Actions` system
    - Dispatch `onUse` and `onUseWith` to registered Lua action handlers matching action IDs or item IDs
    - _Requirements: 13.4_

  - [ ] 9.5 Implement `TalkActions` system
    - Dispatch player speech to registered Lua talk action handlers when text matches a registered command prefix
    - _Requirements: 13.5_

  - [ ] 9.6 Implement XML event loading for all event types
    - Parse event registration XML files and populate `EventDispatcher` registries
    - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5_

  - [ ] 9.7 Implement hot reload: world.reload(ReloadType) with atomic arc-swap at tick boundary
    - Implement `ReloadType` enum: `Scripts`, `Monsters`, `Npcs`, `Actions`, `TalkActions`, `MoveEvents`, `GlobalEvents`, `CreatureEvents`, `All`
    - Load new scripts into a fresh `LuaState` clone on a background thread; validate all scripts load without error
    - The swap MUST only occur at the START of a tick — after the previous tick fully completes and before any script calls in the new tick. A mid-tick swap causes split-brain between in-flight script calls and new event registrations
    - Swap sequence: (1) background thread signals ready, (2) at next tick start: `arc_swap.store(Arc::new(new_state))`, (3) old state dropped after swap
    - Wire `/reload` talk action command to trigger `world.reload(ReloadType::All)`
    - _Requirements: 22.5, 22.6_

  - [ ] 9.8 Checkpoint — cargo check/clippy/fmt pass on event system
    - Ensure all tests pass, ask the user if questions arise.
    - Ensure all tests pass, ask the user if questions arise.

- [ ] 10. Phase 10 — Remaining systems: Chat, VIP, Quests, Raids, Ban, server save
  - [ ] 10.1 Implement `ChatSystem`: all channel types, mute enforcement
    - Implement default channels (Local, World, Trade, Help), private channels, guild channels, party channels
    - Enforce channel-specific speak types and mute rules including configurable mute interval
    - Track mute state per player; reject chat messages while muted
    - _Requirements: 14.1, 14.2, 14.3, 14.4_

  - [ ]* 10.2 Write property test for muted player message rejection (Property 18)
    - **Property 18: Muted Player Message Rejection**
    - **Validates: Requirements 14.4**
    - Test file: `tfs-rust-core/tests/chat.rs`

  - [ ] 10.3 Implement `VipSystem`: load VIP list, online/offline notifications, size enforcement
    - Load VIP list from DB at login; send initial VIP entry list; notify VIP-list holders on login/logout; enforce max size by premium status
    - _Requirements: 16.1, 16.2, 16.3_

  - [ ] 10.4 Implement `BedSystem`: offline training record, wake calculation
    - Record offline training skill and start time in DB on bed sleep; on login calculate elapsed time, apply skill tries up to configured max, wake player
    - Implement `get_bed_by_sleeper` and `set_bed_sleeper` lookups
    - _Requirements: 18.1, 18.2, 18.3_

  - [ ]* 10.5 Write property test for offline training skill calculation (Property 19)
    - **Property 19: Offline Training Skill Calculation**
    - **Validates: Requirements 18.2**
    - Test file: `tfs-rust-core/tests/bed.rs`

  - [ ] 10.6 Implement Quest system
    - Implement quest state tracking via player storage; expose quest log packet builder
    - _Requirements: 10.6 (via Lua API hooks)_

  - [ ] 10.7 Implement Raid system
    - Implement raid scheduling and execution via GlobalEvents; load raid definitions from XML
    - _Requirements: 13.2_

  - [ ] 10.8 Implement Ban system
    - Implement account/IP/player ban checks at login; persist bans to DB; expose ban management via Lua API
    - _Requirements: 5.1_

  - [ ] 10.9 Implement server save: periodic player data flush
    - Implement periodic server save (configurable interval) saving all online players and house data to DB
    - _Requirements: 22.4_

  - [ ] 10.10 Implement `StabilityManager`: error count tracking and recovery handlers
    - Track crash event counts by category (network, script, database, thread) via `DashMap<ErrorCategory, AtomicU64>`
    - Support registering recovery handlers invoked when a category exceeds its threshold
    - Expose counts via server status endpoint
    - _Requirements: 22.2, 22.3_

  - [ ] 10.11 Checkpoint — cargo check/clippy/fmt pass on remaining systems
    - Ensure all tests pass, ask the user if questions arise.


- [ ] 11. Phase 11 — Custom Australis systems: autoloot, dungeon, tasks, rarity scrolls, compression, access tokens
  - [ ] 11.1 Wire autoloot engine hooks
    - Expose all engine hooks required by `data/scripts/custom/autoloot/`: loot event callbacks, player storage read/write, item creation on tile
    - _Requirements: 19.1_

  - [ ] 11.2 Wire dungeon system engine hooks
    - Expose all engine hooks required by `data/lib/dungeon/`: creature spawn, teleport, global event callbacks, zone management
    - _Requirements: 19.2_

  - [ ] 11.3 Wire task system engine hooks
    - Expose all engine hooks required by `data/scripts/custom/tasks/`: kill tracking callbacks, player storage access, task progress packets
    - _Requirements: 19.3_

  - [ ] 11.4 Wire rarity scroll system engine hooks
    - Expose all engine hooks required by `data/scripts/custom/rarity_scrolls/`: item transformation, extended opcode callbacks, item attribute mutation
    - _Requirements: 19.4_

  - [ ] 11.5 Implement packet compression toggle (custom Australis feature)
    - Implement per-connection compression negotiation via extended opcode; integrate with existing flate2 pipeline
    - _Requirements: 4.8, 19.4_

  - [ ] 11.6 Implement access token system (custom Australis feature)
    - Implement access token validation at login for premium/custom feature gating; persist tokens to DB
    - _Requirements: 19.1_

  - [ ] 11.7 Validate all 994 NPC scripts and 604 monster scripts load without error
    - Run `ScriptLoader` against full `data/npc/` and `data/monster/` directories; assert zero load errors
    - _Requirements: 19.5, 19.6, 12.10_

  - [ ] 11.8 Checkpoint — cargo check/clippy/fmt pass on custom systems
    - Ensure all tests pass, ask the user if questions arise.

- [ ] 12. Phase 12 — Testing and hardening: all 20 property tests, integration tests, load testing, benchmarks
  - [ ] 12.1 Write property test for crate dependency invariant (Property 4)
    - **Property 4: Crate Dependency Invariant**
    - **Validates: Requirements 1.2, 1.3, 1.4, 1.5, 1.6, 1.7**
    - Use `cargo_metadata` to parse the workspace dependency graph; assert each crate's deps are a subset of the allowed set
    - Test file: `tests/workspace.rs`

  - [ ] 12.2 Write integration test: full login → play → logout cycle
    - Spin up a test `GameWorld` with in-memory state; simulate a full client login, movement, chat, and logout sequence; assert no errors and correct state transitions
    - _Requirements: 4.1, 4.2, 5.3, 5.4_

  - [ ]* 12.3 Write integration test: protocol compatibility for all 60+ outgoing packets
    - Compare Rust packet bytes against captured TFS reference output for all outgoing packet types
    - Test file: `tests/protocol_compat.rs`
    - _Requirements: 20.2, 4.7_

  - [ ]* 12.4 Write integration test: Lua script compatibility for all NPC and monster scripts
    - Load every script in `data/npc/` (994) and `data/monster/` (604); assert zero errors
    - Test file: `tests/lua_compat.rs`
    - _Requirements: 12.10, 19.5, 19.6_

  - [ ]* 12.5 Write integration test: DB schema compatibility round trip
    - Write a player record with TFS Rust; read it back using TFS schema queries; assert equivalence
    - Test file: `tests/db_compat.rs`
    - _Requirements: 5.4_

  - [ ]* 12.6 Write load test: 200 concurrent players under sustained tick load
    - Simulate 200 concurrent connections sending packets at realistic rates; assert game tick execution time stays below 50ms
    - _Requirements: 21.1_

  - [ ]* 12.7 Write benchmarks for hot paths: get_spectators, pathfind, combat::execute
    - Add `criterion` benchmarks for `get_spectators`, `pathfind`, and `combat::execute`; establish baseline numbers
    - _Requirements: 21.1_

  - [ ] 12.8 Final checkpoint — all tests pass, zero clippy warnings, zero unsafe outside FFI
    - Run `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check --workspace`
    - Verify zero `unsafe` blocks outside `tfs-rust-net/src/rsa.rs` and mlua FFI call sites
    - Ensure all tests pass, ask the user if questions arise.
    - _Requirements: 1.8, 1.9, 21.2_

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- Each phase ends with a `cargo check && cargo clippy && cargo fmt --check` gate — do not proceed to the next phase until the current phase is clean
- Property tests use the `proptest` crate with a minimum of 100 iterations each
- `tfs-rust-common` has zero tfs-rust-* dependencies and must be fully implemented before any other crate
- `tfs-rust-net`, `tfs-rust-db`, and `tfs-rust-content` depend only on `tfs-rust-common` and can be developed in parallel after Phase 0
- `tfs-rust-lua` depends on `tfs-rust-common` + `tfs-rust-core` and must come after Phases 4–5
- `tfs-rust-core` is the integration crate and wires all subsystems together
- The only permitted `unsafe` blocks are in `tfs-rust-net/src/rsa.rs` (RSA FFI) and mlua FFI call sites
