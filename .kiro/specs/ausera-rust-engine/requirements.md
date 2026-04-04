# Requirements Document

## Introduction

TFS Rust is a ground-up rewrite of the Australis TFS 1.4.2 C++ game server (87,826 lines across ~120 files) into a modern, idiomatic Rust engine. The goal is not a line-by-line port — it is a re-architecture that preserves 100% Tibia 8.6 protocol compatibility, 100% Lua API compatibility, and full database schema compatibility, while leveraging Rust's ownership model, type safety, and async concurrency to eliminate entire classes of C++ bugs (segfaults, data races, use-after-free, SQL injection).

The resulting engine is organized as a Cargo workspace of six crates: `tfs-rust-core`, `tfs-rust-net`, `tfs-rust-db`, `tfs-rust-lua`, `tfs-rust-content`, and `tfs-rust-common`.

---

## Glossary

- **TFS Rust**: The Rust game server engine being built by this spec.
- **TFS**: The Forgotten Server 1.4.2 — the C++ reference implementation being replaced.
- **Australis**: The live game server project; TFS Rust is its new engine.
- **OTClient**: The open-source Tibia client used to connect to TFS Rust.
- **OTBM**: Open Tibia Binary Map — the binary map file format.
- **OTB**: Open Tibia Binary — the binary item database format.
- **XTEA**: A symmetric block cipher used to encrypt game protocol packets.
- **RSA**: Asymmetric cipher used during the login handshake.
- **mlua**: Rust crate providing LuaJIT bindings used for the scripting bridge.
- **sqlx**: Async Rust SQL library used for all database access.
- **Tokio**: Async runtime providing the TCP server, timers, and task scheduling.
- **slotmap**: Generational arena crate used for safe entity storage.
- **CreatureId**: A generational index (slotmap key) that uniquely identifies a live creature.
- **ItemId**: A generational index uniquely identifying a live item instance.
- **GameWorld**: The central game state struct owning all entities, the map, and the scheduler.
- **ProtocolGame**: The per-connection struct handling Tibia 8.6 game protocol parsing and sending.
- **ProtocolLogin**: The per-connection struct handling the Tibia 8.6 login protocol.
- **NetworkMessage**: A byte-buffer abstraction for reading and writing binary protocol data.
- **Condition**: A status effect applied to a creature (poison, haste, fire, etc.).
- **CombatParams**: A struct describing a single combat action (type, area, callbacks, conditions).
- **MatrixArea**: A 2D bitmask defining the area of effect for spells and combat.
- **Vocation**: A player character class (e.g., Knight, Druid, Sorcerer, Paladin).
- **Spawn**: A map-defined point that periodically creates monsters.
- **House**: A player-ownable in-game building with access control.
- **Party**: A group of players sharing experience and combat bonuses.
- **Guild**: A persistent player organization with ranks and a message of the day.
- **Market**: The in-game player-to-player trading system.
- **Depot**: Per-town player item storage accessible from depot chests.
- **Lua API**: The set of Lua classes and methods exposed by the engine to game scripts.
- **EventCallback**: A Lua-registered callback invoked by the engine on specific game events.
- **PropStream**: A binary reader used for item/condition deserialization from DB blobs.
- **PropWriteStream**: A binary writer used for item/condition serialization to DB blobs.
- **WildcardTree**: A trie data structure used for player name prefix autocomplete.
- **StabilityManager**: A subsystem tracking crash statistics, recovery handlers, and resource monitoring.
- **MapBackend**: A trait abstracting map storage (OTBM file or database-backed).
- **SectorCoord**: A 32×32 tile region coordinate used by the database map backend.

---

## Requirements

### Requirement 1: Workspace and Crate Architecture

**User Story:** As a Rust developer, I want a well-structured Cargo workspace, so that each subsystem compiles independently and dependency boundaries are enforced by the type system.

#### Acceptance Criteria

1. THE TFS Rust workspace SHALL define exactly six member crates: `tfs-rust-common`, `tfs-rust-content`, `tfs-rust-db`, `tfs-rust-net`, `tfs-rust-lua`, and `tfs-rust-core`.
2. THE `tfs-rust-common` crate SHALL contain all shared types, enums, constants, `Position`, `PropStream`, `PropWriteStream`, and error types, and SHALL NOT depend on any other TFS Rust crate.
3. THE `tfs-rust-content` crate SHALL depend only on `tfs-rust-common` and SHALL NOT depend on `tfs-rust-core`, `tfs-rust-net`, `tfs-rust-db`, or `tfs-rust-lua`.
4. THE `tfs-rust-db` crate SHALL depend only on `tfs-rust-common` and SHALL NOT depend on `tfs-rust-core`, `tfs-rust-net`, `tfs-rust-lua`, or `tfs-rust-content`.
5. THE `tfs-rust-net` crate SHALL depend only on `tfs-rust-common` and SHALL NOT depend on `tfs-rust-core`, `tfs-rust-db`, `tfs-rust-lua`, or `tfs-rust-content`.
6. THE `tfs-rust-lua` crate SHALL depend on `tfs-rust-common` and `tfs-rust-core` and SHALL NOT depend on `tfs-rust-net` or `tfs-rust-db`.
7. THE `tfs-rust-core` crate SHALL depend on `tfs-rust-common`, `tfs-rust-content`, `tfs-rust-db`, and `tfs-rust-lua`.
8. THE TFS Rust workspace SHALL compile with zero `unsafe` blocks except those required for FFI boundaries with LuaJIT and the RSA crate.
9. WHEN `cargo build` is executed on the workspace root, THE Build System SHALL produce a single `tfs-rust` binary with no compilation errors or warnings.

---

### Requirement 2: Async Runtime and Game Loop

**User Story:** As a server operator, I want a stable, tick-based game loop, so that game logic executes deterministically at a fixed rate while I/O remains non-blocking.

#### Acceptance Criteria

1. THE GameWorld SHALL execute all game state mutations on a single logical game thread driven by a `tokio::time::interval` of 50 milliseconds per tick.
2. WHEN the game tick fires, THE GameWorld SHALL process creature movement, creature think, condition ticks, decay ticks, and light cycle updates in that order within the same tick.
3. THE TFS Rust Server SHALL use Tokio as its async runtime for all TCP I/O, timer scheduling, and background task execution.
4. WHEN a network packet arrives from a client, THE ProtocolGame SHALL decode the packet on the Tokio I/O task and forward a typed command to the GameWorld via a `tokio::sync::mpsc` channel.
5. WHEN a database operation is required during a game tick, THE GameWorld SHALL dispatch the operation to a Tokio task via `tokio::spawn` and SHALL NOT block the game thread awaiting the result.
6. THE Scheduler SHALL support delayed task execution by accepting a duration and a closure, implemented via `tokio::time::sleep_until` followed by channel dispatch to the game thread.
7. WHEN the server receives SIGINT or SIGTERM, THE SignalHandler SHALL initiate a graceful shutdown that saves all online players, closes all connections, and exits within 30 seconds.

---

### Requirement 3: Configuration System

**User Story:** As a server operator, I want to configure the server via the existing `config.lua` file, so that I do not need to change my configuration workflow when switching from TFS to TFS Rust.

#### Acceptance Criteria

1. THE ConfigManager SHALL load server configuration by executing `config.lua` through the mlua Lua runtime at startup.
2. THE ConfigManager SHALL expose typed accessors `get_string(key)`, `get_number::<T>(key)`, and `get_bool(key)` for all configuration values.
3. IF a required configuration key is absent from `config.lua`, THEN THE ConfigManager SHALL return a descriptive error identifying the missing key and halt startup.
4. THE ConfigManager SHALL support all configuration keys present in the TFS 1.4.2 `config.lua` schema without modification to the config file.

---

### Requirement 4: Network Protocol Layer

**User Story:** As a player, I want to connect to TFS Rust using OTClient, so that my gameplay experience is identical to the C++ server.

#### Acceptance Criteria

1. THE ProtocolLogin SHALL implement the Tibia 8.6 login protocol, accepting connections on the configured login port, performing RSA decryption of the first message, and returning a character list or error message.
2. THE ProtocolGame SHALL implement the Tibia 8.6 game protocol, accepting connections on the configured game port and maintaining per-connection XTEA encryption state.
3. THE XTEA Module SHALL encrypt and decrypt 8-byte blocks using the 128-bit session key negotiated during login, producing output byte-for-byte identical to the TFS C++ implementation.
4. THE RSA Module SHALL decrypt the 128-byte RSA-encrypted login block using the server's private key, producing output byte-for-byte identical to the TFS C++ implementation.
5. THE NetworkMessage SHALL support reading and writing `u8`, `u16`, `u32`, `u64`, `String` (length-prefixed), and `Position` values in little-endian byte order.
6. THE ProtocolGame SHALL parse all 50+ incoming client packet opcodes defined in the Tibia 8.6 protocol, dispatching each to the corresponding GameWorld command.
7. THE ProtocolGame SHALL send all 60+ outgoing server packet opcodes defined in the Tibia 8.6 protocol, producing byte sequences identical to those produced by TFS 1.4.2.
8. WHERE zlib compression is enabled in the server configuration, THE NetworkMessage SHALL compress outgoing packets using flate2 and decompress incoming packets using flate2.
9. THE ProtocolStatus SHALL respond to status queries on the configured status port with a valid XML status document.
10. WHEN a client connection is closed unexpectedly, THE Connection SHALL clean up the associated player session and release all held resources within one game tick.
11. THE ProtocolGame SHALL support the OTCv8 extended opcode protocol, dispatching `parseExtendedOpcode` messages to registered Lua `PacketHandler` callbacks.
12. WHEN a Lua `PacketHandler` callback for an extended opcode requires database access, it SHALL use only async DB functions (`db.asyncQuery`, `db.asyncStoreQuery`); synchronous DB calls SHALL NOT be available in the extended opcode handler context, preventing game tick stalls.

---

### Requirement 5: Database Layer

**User Story:** As a server operator, I want all database access to use prepared statements over an async connection pool, so that SQL injection is impossible and the game thread is never blocked on I/O.

#### Acceptance Criteria

1. THE DbPool SHALL establish an async MariaDB connection pool using sqlx at startup, with configurable minimum and maximum connection counts.
2. THE DbPool SHALL use only sqlx prepared statements for all queries; string-concatenated SQL SHALL NOT appear anywhere in the codebase.
3. THE PlayerData Module SHALL load a complete player record (stats, inventory, skills, storage, conditions, spells, vip list, guild, outfits, mounts) from the database using the same schema as TFS 1.4.2.
4. THE PlayerData Module SHALL save a complete player record to the database using the same schema as TFS 1.4.2, such that a database created by TFS Rust is readable by TFS 1.4.2 and vice versa.
5. THE Migrations Module SHALL check the database schema version at startup and apply any pending migrations before the server accepts connections.
6. THE Market Module SHALL implement all market operations (create offer, accept offer, cancel offer, browse by item, browse own offers, browse history) using async prepared statements.
7. THE MapSerialize Module SHALL load and save house item contents and house ownership information to the database using `PropStream` and `PropWriteStream` for blob serialization.
8. WHEN a database operation fails due to a transient connection error, THE DbPool SHALL retry the operation up to 3 times with exponential backoff before returning an error.
9. THE PlayerData Module SHALL serialize item attributes and condition data to binary blobs using `PropWriteStream` and deserialize them using `PropStream`, producing blobs byte-compatible with TFS 1.4.2.

---

### Requirement 6: Content Loaders

**User Story:** As a content developer, I want TFS Rust to load all existing data files (OTBM, OTB, XML) without modification, so that I do not need to re-export or convert any game content.

#### Acceptance Criteria

1. THE OtbmLoader SHALL parse the OTBM binary map format, loading all tile areas, tile items, spawns, houses, towns, and waypoints into the GameWorld.
2. THE OtbLoader SHALL parse the OTB binary item database format, populating the ItemDatabase with all item type definitions.
3. THE ItemDatabase SHALL additionally parse `items.xml` to load extended item attributes not present in the OTB file.
4. THE ContentLoader SHALL parse `monsters/` XML files to populate the MonsterType registry with all monster definitions including loot tables and spell lists.
5. THE ContentLoader SHALL parse `vocations.xml`, `outfits.xml`, `mounts.xml`, and `groups.xml` to populate their respective registries.
6. THE OtbmLoader SHALL parse waypoints from the OTBM file and expose them via `world.get_waypoint_by_name(name)`.
7. THE OtbmLoader SHALL parse town definitions from the OTBM file and expose them via the Town registry.
8. IF any content file is malformed or missing a required field, THEN THE ContentLoader SHALL log a descriptive error including the file path and line/offset, and SHALL halt startup.
9. THE ContentLoader SHALL load all content files concurrently where files are independent, completing the full content load in less time than sequential loading.

---

### Requirement 7: Core World and Entity Storage

**User Story:** As a game developer, I want all entities stored in a generational arena, so that dangling references to dead creatures or items are impossible at compile time.

#### Acceptance Criteria

1. THE GameWorld SHALL store all live creatures in a `slotmap::SlotMap<CreatureId, CreatureKind>` generational arena.
2. THE GameWorld SHALL store all live items not inside containers in a `slotmap::SlotMap<ItemId, Item>` generational arena.
3. THE CreatureKind enum SHALL have variants `Player(Player)`, `Monster(Monster)`, and `Npc(Npc)`, replacing the C++ virtual inheritance hierarchy.
4. WHEN a creature is removed from the world, THE GameWorld SHALL invalidate its `CreatureId` such that any subsequent lookup by that key returns `None`.
5. THE Map SHALL implement a quadtree spatial index equivalent to the TFS `QTreeNode`, supporting `get_spectators(pos, range)` queries that return all `CreatureId` values within the specified range.
6. THE Map SHALL provide `get_tile(pos)` and `get_tile_mut(pos)` accessors returning `Option<&Tile>` and `Option<&mut Tile>` respectively.
7. THE WildcardTree SHALL implement a trie supporting prefix-based player name lookup, used by `world.get_player_by_name_wildcard(prefix)`.
8. THE Position struct SHALL implement `distance_to`, `get_direction_to`, `offset`, and range-check methods with behavior identical to the TFS C++ `Position` implementation.

---

### Requirement 8: Creature System

**User Story:** As a game developer, I want a complete creature system covering players, monsters, and NPCs, so that all creature behaviors from TFS are preserved.

#### Acceptance Criteria

1. THE CreatureBase struct SHALL contain all fields shared across creature kinds: id, name, position, direction, health, max_health, outfit, speed, skull, conditions, walk_queue, follow target, attack target, and damage tracking.
2. THE Player struct SHALL decompose the TFS god-object `Player` into sub-structs `PlayerInventory`, `PlayerSkills`, `PlayerEconomy`, and `PlayerSocial`, while exposing the same logical interface.
3. WHEN a player gains experience, THE Player SHALL check for level advancement and, if a level-up occurs, recalculate max health, max mana, and capacity according to the vocation formula.
4. THE Monster struct SHALL implement the full TFS AI loop in native Rust: target selection, flee behavior, friend/target list management, idle detection, return-to-spawn walk, and per-think sub-routines (target, yell, defense).
5. THE Monster AI loop SHALL invoke the Lua `onThink` creature event ONLY for monsters whose `registered_events` list contains `"onThink"`. Monsters without this registration SHALL NOT cross the FFI boundary during the creature-think tick.
6. THE Npc struct SHALL implement the NPC event model: `on_appear`, `on_disappear`, `on_say`, `on_buy`, `on_sell`, `on_check_item`, and `on_close_channel`, each dispatching to the NPC's Lua script via the `NpcEventsHandler`.
7. THE Party struct SHALL implement shared experience distribution using the Australis custom formula, party leadership transfer, invitation management, and shared experience toggle.
8. WHEN a creature dies, THE GameWorld SHALL execute the death sequence: drop corpse, drop loot, attribute experience to killers by damage ratio, fire `onKill` and `onDeath` creature events, and schedule corpse decay.
9. THE Creature system SHALL implement the summon model: a creature may have a master `CreatureId`, and `is_summon()` SHALL return true when a master is set.

---

### Requirement 9: Combat System

**User Story:** As a game developer, I want a complete combat system, so that all damage formulas, conditions, spells, and weapons produce results identical to TFS 1.4.2.

#### Acceptance Criteria

1. THE Combat Module SHALL implement `execute(world, attacker, params)` as the single entry point for all combat actions, dispatching to health, mana, condition, or dispel sub-functions based on `CombatParams.combat_type`.
2. THE Combat Module SHALL implement `can_attack(world, attacker, target)` enforcing all TFS PvP rules: protection zone, skull mode, secure mode, and world type (PvP/No-PvP/Hardcore).
3. THE ConditionData enum SHALL have variants for all TFS condition subtypes: `Damage`, `Speed`, `Outfit`, `Light`, `Regeneration`, `Soul`, `Attributes`, `SpellCooldown`, `SpellGroupCooldown`, and `Generic`.
4. WHEN a condition is applied to a creature that already has a condition of the same type and id, THE Condition System SHALL merge the new condition into the existing one using the same logic as TFS `addCondition`.
5. THE MatrixArea struct SHALL implement `flip`, `mirror`, and `rotate90` transforms, producing results identical to the TFS C++ `MatrixArea` implementation.
6. THE Weapon Module SHALL implement melee, distance, and wand weapon damage calculations including element damage, producing results identical to TFS 1.4.2 formulas.
7. THE Spell Module SHALL implement instant spells and rune spells, including cooldown enforcement, mana cost, soul cost, level requirement, and vocation restriction checks.
8. WHEN a spell or combat action defines a `MatrixArea`, THE Combat Module SHALL apply the effect to all tiles within the area relative to the caster or target position.

---

### Requirement 10: Item System

**User Story:** As a game developer, I want a complete item system, so that all item types, containers, depots, and decay behave identically to TFS 1.4.2.

#### Acceptance Criteria

1. THE Item struct SHALL store item attributes in a typed `ItemAttributes` struct with optional fields for attack, defense, armor, charges, duration, text, description, and a `HashMap<String, AttributeValue>` for custom attributes.
2. THE Container struct SHALL implement `add_item`, `remove_item`, `get_item(index)`, `iter()`, `capacity()`, and `empty_slots(recursive)` with behavior identical to TFS.
3. THE ContainerKind enum SHALL have variants `Regular`, `DepotChest { max_items }`, `DepotLocker { depot_id }`, `Inbox`, and `StoreInbox`, replacing the C++ subclass hierarchy.
4. THE DecayManager SHALL track all decaying items and, on each game tick, advance their timers and transform or remove items whose duration has elapsed.
5. THE Item serialization system SHALL use `PropWriteStream` to serialize item attributes to binary blobs and `PropStream` to deserialize them, producing blobs byte-compatible with TFS 1.4.2.
6. WHEN an item is moved between a tile, container, or player slot, THE GameWorld SHALL fire the appropriate `MoveEvent` callbacks (onAddItem, onRemoveItem, onMoveItem).

---

### Requirement 11: World and Map System

**User Story:** As a server operator, I want the map system to support the full TFS feature set including pathfinding, houses, and spawns, so that all map-dependent gameplay works correctly.

#### Acceptance Criteria

1. THE Map SHALL implement A* pathfinding via `pathfind(from, to, params)` returning `Option<Vec<Direction>>`, with behavior identical to the TFS `AStarNodes` implementation.
2. THE Map SHALL implement `can_throw_to(from, to)` and `is_sight_clear(from, to)` line-of-sight checks with behavior identical to TFS.
3. THE House system SHALL implement owner assignment, access list management (guest/subowner/door), player kick, bed tracking, and tile ownership, with data persisted to the database.
4. THE SpawnManager SHALL load spawn definitions from the OTBM file, create initial monsters at startup, and respawn monsters after their configured interval following death.
5. THE Tile struct SHALL implement `query_add`, `query_remove`, `add_thing`, `remove_thing`, `top_visible_thing`, and zone/flag accessors with behavior identical to TFS.
6. THE HouseTile variant SHALL extend Tile behavior with a house reference, used by the house access control system.
7. THE MapBackend trait SHALL abstract map storage, with `OtbmBackend` as the default implementation loading the full map at startup.

---

### Requirement 12: Lua Scripting Bridge

**User Story:** As a content developer, I want all existing NPC scripts, monster scripts, and custom system scripts to load and execute without modification, so that I do not need to rewrite any game content for the new engine.

#### Acceptance Criteria

1. THE LuaState SHALL initialize a LuaJIT runtime via mlua and register all 30+ Lua classes as userdata metatables before loading any script files.
2. THE Lua API SHALL expose all ~500 methods across all registered classes with signatures and behavior identical to TFS 1.4.2 `luascript.cpp`.
3. THE ScriptLoader SHALL discover and load scripts in the order: `data/lib/` → `data/events/` → `data/scripts/` → `data/monster/` → `data/npc/`, matching the TFS loading pipeline.
4. THE NpcScriptInterface SHALL provide NPC-specific Lua functions (`doNpcSetCreatureFocus`, `openShopWindow`, `closeShopWindow`, `doSellItem`, `doBuyItem`, `doSendCreatureSay`) that are only available within NPC script contexts.
5. THE EventCallback system SHALL allow Lua scripts to register callbacks for all TFS event types (onLogin, onLogout, onThink, onDeath, onKill, onPrepareDeath, onMoveItem, onMoveCreature, onSay, onExtendedOpcode, etc.).
6. WHEN a Lua script raises an error, THE LuaState SHALL catch the error via `std::panic::catch_unwind` or mlua error handling, log the error with script name and line number, and continue server operation without crashing.
7. THE Lua Database table SHALL expose `db.query`, `db.asyncQuery`, `db.storeQuery`, `db.asyncStoreQuery`, `db.escapeString`, `db.tableExists`, and `db.lastInsertId` to Lua scripts, maintaining API compatibility with TFS.
8. THE Lua ConfigManager table SHALL expose `configManager.getString`, `configManager.getNumber`, and `configManager.getBoolean` to Lua scripts.
9. THE addEvent and stopEvent global Lua functions SHALL schedule and cancel delayed Lua callbacks using the Tokio scheduler, with behavior identical to TFS.
10. FOR ALL valid Lua scripts that load successfully on TFS 1.4.2, THE ScriptLoader SHALL load the same scripts successfully on TFS Rust without modification.

---

### Requirement 13: Event System

**User Story:** As a content developer, I want all TFS event hooks to fire correctly, so that all custom game logic implemented in Lua continues to work.

#### Acceptance Criteria

1. THE CreatureEvents system SHALL fire `onLogin`, `onLogout`, `onThink`, `onPrepareDeath`, `onDeath`, `onKill`, `onAdvance`, `onModalWindow`, `onTextEdit`, `onHealthChange`, `onManaChange`, `onExtendedOpcode`, and `onMoveItem` events to all registered Lua callbacks.
2. THE GlobalEvents system SHALL fire `onStartup`, `onShutdown`, `onRecord`, `onTime`, `onTimer`, and `onPeriodChange` events to all registered Lua callbacks.
3. THE MoveEvents system SHALL fire `onAddItem`, `onRemoveItem`, `onMoveItem`, `onStepIn`, and `onStepOut` events when items or creatures interact with registered tiles or item types.
4. THE Actions system SHALL dispatch `onUse` and `onUseWith` events to registered Lua action handlers when players use items with matching action IDs or item IDs.
5. THE TalkActions system SHALL dispatch player speech to registered Lua talk action handlers when the spoken text matches a registered command prefix.
6. WHEN multiple Lua callbacks are registered for the same event, THE Event System SHALL invoke them in registration order and SHALL stop the chain if any callback returns `false`.

---

### Requirement 14: Chat and Channel System

**User Story:** As a player, I want all chat channels to work correctly, so that I can communicate with other players using the same channels as the C++ server.

#### Acceptance Criteria

1. THE ChatSystem SHALL implement all TFS channel types: default channels (Local, World, Trade, Help), private channels, guild channels, and party channels.
2. WHEN a player opens a channel, THE ChatSystem SHALL send the channel dialog listing all available channels the player may join.
3. THE ChatSystem SHALL enforce channel-specific speak types and mute rules, including the configurable mute interval for repeated messages.
4. WHEN a player is muted, THE ChatSystem SHALL track the mute state per player and reject chat messages until the mute expires.

---

### Requirement 15: Guild System

**User Story:** As a guild leader, I want all guild features to work correctly, so that guild management, wars, and communication are preserved.

#### Acceptance Criteria

1. THE GuildSystem SHALL load guild data (name, motd, ranks, members) from the database at player login.
2. WHEN a player logs in and belongs to a guild, THE GameWorld SHALL send the guild MOTD to the player.
3. THE GuildSystem SHALL support guild war state tracking, exposing `is_in_war(player_a, player_b)` for PvP skull calculations.

---

### Requirement 16: VIP System

**User Story:** As a player, I want my VIP list to work correctly, so that I can track the online status of my friends.

#### Acceptance Criteria

1. THE VipSystem SHALL load a player's VIP list from the database at login and send the initial VIP entry list to the client.
2. WHEN a VIP-listed player comes online or goes offline, THE VipSystem SHALL notify all players who have that player on their VIP list.
3. THE VipSystem SHALL enforce the maximum VIP list size based on the player's premium status.

---

### Requirement 17: Market System

**User Story:** As a player, I want the in-game market to work correctly, so that I can buy and sell items with other players.

#### Acceptance Criteria

1. THE Market System SHALL allow players to create buy and sell offers for any tradeable item type.
2. WHEN a player creates an offer, THE Market System SHALL deduct the item (for sell offers) or gold (for buy offers) from the player's depot immediately.
3. WHEN a matching offer exists, THE Market System SHALL automatically complete the trade, crediting items and gold to the respective depots.
4. THE Market System SHALL enforce the maximum number of active offers per player based on premium status.
5. THE Market System SHALL persist all offers to the database and restore them on server restart.

---

### Requirement 18: Offline Training and Bed System

**User Story:** As a player, I want offline training and bed sleeping to work correctly, so that I can train skills while offline.

#### Acceptance Criteria

1. WHEN a player sleeps in a bed, THE BedSystem SHALL record the player's offline training skill and start time in the database.
2. WHEN a player logs in after sleeping in a bed, THE BedSystem SHALL calculate the training time elapsed, apply skill tries up to the configured maximum, and wake the player.
3. THE BedSystem SHALL track which bed a player is sleeping in via `get_bed_by_sleeper` and `set_bed_sleeper` lookups.

---

### Requirement 19: Custom Australis Systems

**User Story:** As a player, I want all Australis-specific custom systems (autoloot, dungeon, task system, rarity scrolls) to work correctly, so that the custom gameplay features are preserved.

#### Acceptance Criteria

1. THE Lua API SHALL expose all engine hooks required by the autoloot system (`data/scripts/custom/autoloot/`), including loot event callbacks and player storage access.
2. THE Lua API SHALL expose all engine hooks required by the dungeon system (`data/lib/dungeon/`), including creature spawn, teleport, and global event callbacks.
3. THE Lua API SHALL expose all engine hooks required by the task system (`data/scripts/custom/tasks/`), including kill tracking and player storage access.
4. THE Lua API SHALL expose all engine hooks required by the rarity scroll system (`data/scripts/custom/rarity_scrolls/`), including item transformation and extended opcode callbacks.
5. FOR ALL 994 NPC scripts in `data/npc/`, THE ScriptLoader SHALL load each script without error on TFS Rust.
6. FOR ALL 604 monster scripts in `data/monster/`, THE ScriptLoader SHALL load each script without error on TFS Rust.

---

### Requirement 20: Protocol Compatibility Verification

**User Story:** As a QA engineer, I want a protocol compatibility test suite, so that I can verify TFS Rust produces byte-identical output to TFS for all packet types.

#### Acceptance Criteria

1. THE XTEA Module SHALL pass a round-trip property test: FOR ALL valid 8-byte aligned byte sequences and 128-bit keys, `xtea::decrypt(xtea::encrypt(data, key), key)` SHALL equal the original data.
2. THE NetworkMessage SHALL pass a round-trip property test: FOR ALL sequences of typed values written via `write_*` methods, reading them back via the corresponding `read_*` methods SHALL return the original values.
3. THE PropStream and PropWriteStream SHALL pass a round-trip property test: FOR ALL item attribute sets, `PropStream::read(PropWriteStream::write(attrs))` SHALL produce an attribute set equal to the original.
4. THE Combat damage formulas SHALL produce results equal to the TFS 1.4.2 C++ formulas for all valid input ranges, verified by a model-based test comparing TFS Rust output against a reference implementation.
5. THE A* pathfinding implementation SHALL produce paths equal in length to the TFS C++ implementation for all valid start/end position pairs on the loaded map.

---

### Requirement 21: Performance and Safety

**User Story:** As a server operator, I want TFS Rust to handle 200+ concurrent players with equal or better performance than TFS, with zero memory safety violations.

#### Acceptance Criteria

1. THE TFS Rust Server SHALL sustain 200 concurrent connected players with a game tick execution time below 50 milliseconds, measured on reference hardware equivalent to the current production server.
2. THE TFS Rust Server SHALL produce zero segmentation faults, zero data races, and zero use-after-free errors under any load condition, guaranteed by the Rust ownership model and the absence of `unsafe` code outside FFI boundaries.
3. THE GameWorld SHALL use `dashmap::DashMap` for concurrent player name and GUID lookups that may be accessed from both the game thread and async I/O tasks.
4. THE TFS Rust Server SHALL use `parking_lot::Mutex` and `parking_lot::RwLock` in preference to `std::sync` equivalents for all shared state requiring synchronization.
5. WHEN the server is under load, THE TFS Rust Server SHALL log structured diagnostics via the `tracing` crate at configurable verbosity levels without impacting game tick timing.
6. THE GameWorld tick loop SHALL call `lua.gc(LuaGCMode::Step, step_size)` every 5 ticks to perform incremental LuaJIT garbage collection, preventing stop-the-world GC pauses from accumulating stale Lua wrapper objects.
7. THE Map spectator cache SHALL cache the creature list per 32×32 tile sector leaf node and invalidate it only when a creature enters or leaves that sector, avoiding redundant quadtree traversals on every `get_spectators` call.
8. WHEN a connection's outgoing send queue exceeds the configured byte limit, THE OutputMessage Queue SHALL drop stale visual-only packets (map description chunks, magic effects, creature movement) before dropping critical packets (stats, inventory, text messages).
9. IF a connection's send queue remains full for more than 3 consecutive ticks, THE Connection SHALL be closed and the player session cleaned up.
10. WHEN a game tick body exceeds 45 milliseconds, THE TickWatchdog SHALL emit a `tracing::warn!` log entry identifying the elapsed time. WHEN a tick exceeds 50 milliseconds, THE StabilityManager SHALL record a `TickOverrun` error event.
11. ALL insertions and removals from `player_by_name` and `player_by_guid` DashMaps SHALL be performed exclusively on the game thread. Tokio I/O tasks SHALL only perform read operations on these maps.

---

### Requirement 23: PropStream Golden Blob Compatibility

**User Story:** As a server operator, I want PropStream to correctly parse all existing player item and condition data from the live database, so that migrating to TFS Rust does not corrupt any player data.

#### Acceptance Criteria

1. BEFORE Phase 2 implementation begins, a minimum of 1,000 real item blobs and 500 real condition blobs SHALL be extracted from the live Australis MariaDB and committed as binary test fixtures in `tfs-rust-common/tests/fixtures/blobs/`.
2. THE PropStream implementation SHALL correctly deserialize 100% of the golden blob fixtures without error.
3. THE PropWriteStream + PropStream round-trip SHALL produce output byte-identical to the original blob for all golden fixtures.
4. THE golden blob test suite SHALL be run as part of the Phase 0 checkpoint, before any player data load/save code is written in Phase 2.

---

### Requirement 22: Stability and Error Recovery

**User Story:** As a server operator, I want the server to recover gracefully from script errors and log crash statistics, so that a single bad script does not bring down the server.

#### Acceptance Criteria

1. WHEN a Lua script raises an unhandled error, THE LuaState SHALL catch the error, log it with full context (script name, line, error message), and continue processing the current game tick.
2. THE StabilityManager SHALL track crash event counts by category (network, script, database, thread) and expose them via the server status endpoint.
3. THE StabilityManager SHALL support registering recovery handlers that are invoked when a subsystem exceeds its configured error threshold.
4. WHEN the server is shut down gracefully, THE GameWorld SHALL save all online player data to the database before closing connections.
5. THE server SHALL support hot reload of scripts, monsters, NPCs, actions, talk actions, move events, global events, and creature events via `world.reload(ReloadType)` without restarting the server process.
6. WHEN a hot reload is triggered, THE LuaState SHALL load new scripts into a fresh state, validate all scripts load without error, then atomically swap the active state using `arc-swap`, with zero downtime for connected players.
