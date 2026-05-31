# TODO

## Bcrypt password migration — done
- [x] `tfs-rust-db/src/password.rs`: verify, hash, upgrade-on-login, unit tests
- [x] Refactor `account.rs` auth paths; wire `PasswordHashConfig` through net + `run_server`
- [x] SQLx migration widen `accounts.password`; update `schema.sql`, `config.lua`
- [x] `cargo test` / `cargo check` / clippy

## Config-and-save plan — Step 2 (run_server wiring)
- [x] Wire `run_server::run` listener bind addresses to `NetConfig` with env overrides preserved.
- [x] Wire database URL resolution to `DbConfig` when `DATABASE_URL` is unset.
- [x] Keep `TFS_GAME_PORT` / `TFS_PUBLIC_IP` env override behavior; fallback to `NetConfig`.
- [x] Add warning path for differing `statusProtocolPort` (listener not yet implemented).
- [x] Run `cargo check -p tfs-rust-core`.
- [x] Mark Step 2 complete in `tasks/config-and-save-plan.md`.

## Phase C (inventory / Lua) — done
- Runtime equipment slots, capacity, `internal_move_item` container↔inventory, quick-equip, equip/deequip `EventDispatcher` hooks, Lua `ItemRef` + `Player` inventory methods (`register_game_lua_item_hooks` + script cookie in `login.rs`).

## P1 — Player inventory cylinder queries — done
- [x] `player_query_remove`, `player_query_max_count`, `player_query_destination` (`player_inventory_query_add.rs`)
- [x] Wire `resolve_move_destination` + `internal_move_item` query chain (`container_ops.rs`, `game_world.rs`)
- [x] Unit tests: `player_max_count_index`, slot range constants
- [x] Update `docs/INVENTORY_STATUS.md`

## Terrain look parity — done
- [x] `LookTarget` enum + `internal_get_thing_look` (`STACKPOS_LOOK`, `game.cpp` ~223)
- [x] `Tile::top_visible_look_target` / `getTopVisibleThing` (`tile.cpp` ~322)
- [x] `player_look_at` ground + immovable terrain descriptions; `can_see_position` gate
- [x] Unit tests: `tile::look_tests`, `item_look::ground_water_description`
- [x] Update `docs/INVENTORY_STATUS.md`, `tasks/lessons.md`

## P4 — Depot & inbox runtime — done
- [x] Item typing: `is_depot()`, `depot_id` attributes, `DEPOT` tile flag, item constants
- [x] `player_depot.rs`: getInbox, getDepotChest, getDepotLocker, getMaxDepotItems, isNearDepotBox, last_depot_id
- [x] `load_depot_table` + `load_inbox_table`; login wiring; store inbox `ContainerType::StoreInbox`
- [x] Depot locker open branch in `container_ui.rs`; locker/inbox `queryAdd` rules
- [x] `DepotIsFull` in `container_ops.rs`; `depotFreeLimit` / `depotPremiumLimit` config
- [x] Live depot/inbox serialization in `game_world_save.rs`
- [x] Depot-owner container refresh in `player_inventory_notifications.rs`
- [x] Unit tests + `docs/INVENTORY_STATUS.md` update

## Ladder / grate UseItem teleport — done
- [x] `Tile::item_id_for_use` (`getUseItem` parity) + sprite-id fallback on map tiles
- [x] Defer `UseItem` on `nextAction` (not silent game-loop drop / cancel)
- [x] Immediate flush for `UseItem` / teleport packets
- [x] Unit tests: `tile::look_tests`, `game_loop::timed_action_gate_tests`

## P5 — Lua Container + Inventory API — done
- [x] Tranche 0: `find_item_of_type`, `ScriptContext` cylinder reads, `lua_script_*`, `fire_on_player_equip*`
- [x] Tranche 1: Player `getItemById`, container id round-trip bindings
- [x] Tranche 2: `getDepotChest`, `getInbox`, expanded `addItem`
- [x] Tranche 3: `Container` userdata (inherits Item)
- [x] Tranche 4: `item:moveTo`, `item:remove`
- [x] Tranche 5: item parent/position/attrs
- [x] Tranche 6: MoveEvents XML loader + equip dispatch (defer tile MoveEvents + `MoveEvent():register()`)
- [x] `docs/INVENTORY_STATUS.md`, `tasks/lessons.md` updated

## Phase D.1 — Generalize walk engine (Monster/NPC) — done
- [x] Route walk timing through `CreatureBase`; `step_speed_for_walk` (player clamp vs base speed)
- [x] `tile_query_add_monster` / `tile_query_add_npc` / `tile_query_add_creature`; `creature_can_stand_for_pathfind`
- [x] `internal_move_creature_step` (player height walk only); spectator `0x6D` via `creature_wire_id`
- [x] `creature_queue_walk_step` + `monster_walk_step_broadcasts_spectator_move` test
- [x] `cargo test -p tfs-rust-core monster_walk`

## Phase D.5 — Follow-on-target-move repath — done
- [x] `has_follow_path` gate in `monster_on_follow_creature_moved` (`creature.cpp` ~619)
- [x] Acceptance test `monster_repaths_when_follow_target_moves`
- [x] Fix `compute_look_toward_target` offset args (lesson #25 contract)
- [x] `cargo test --workspace` green

## Throw destination validation parity (B.5)
- [ ] Confirm C++ reference behavior for `Game::playerMoveItem` throw gating.
- [ ] Add Rust throw-destination validation before `internal_move_item`.
- [ ] Verify compile for `tfs-rust-core` after patch.

## Bugfix: inventory container move disappearance
- [ ] Confirm failure path in `internal_move_item` for `* -> Container` branches.
- [ ] Ensure destination can accept non-merge insert before source removal.
- [ ] Keep behavior parity for merge moves and normal successful inserts.
- [ ] Run `cargo check -p tfs-rust-core`.

## Audit: item parsers parity
- [x] Audit `items.otb` parser attribute/flag coverage vs TFS C++.
- [x] Audit `items.xml` parser key coverage vs TFS C++.
- [x] Write findings report with severity and recommended fixes.

## Implement: item parsers full parity pass
- [ ] Expand OTB `apply_attr` coverage for parity-relevant `itemattrib_t` fields.
- [ ] Expand XML typed key mapping (`apply_xml_attribute`) for runtime-relevant keys and aliases.
- [ ] Parse non-empty `<attribute>` blocks (`Event::Start`) including nested `field` sub-attributes.
- [ ] Switch container identity truth to OTB group (`group == ITEM_GROUP_CONTAINER`).
- [ ] Add parity diagnostics for unknown keys/types and duplicate XML item definitions.
- [ ] Add regression tests for OTB decode coverage, XML parity keys, nested attribute parsing, and container truth.
- [ ] Run `cargo test -p tfs-rust-content`, `cargo check`, and `cargo test -p tfs-rust-core`.

## Phase A0 — Protocol version scaffolding (Track A)
- [x] `ProtocolVersion`, `ProtocolCaps`, `ProtocolCaps::for_version` in `tfs-rust-common`
- [x] `clientVersion` in `config.lua.dist`; `resolve_protocol_version` + `TFS_PROTOCOL_VERSION` override
- [x] Thread `protocol_version` + `protocol_caps` on `GameWireConfig` / `LoginWireConfig`
- [x] Unit tests: `protocol_caps.rs` (1098 matrix + 772 invariants) + config tests
- [x] `cargo check --workspace`, `cargo test -p tfs-rust-common protocol_caps`, `cargo clippy`

## Phase A1 — Codec seam (10.98 only, Track A)
- [x] `codec/{mod,wire,v1098}.rs`: `ProtocolCodec`, `Codec1098`, `Codec` enum, `from_version` (772 rejected until A5)
- [x] `PlayerStatsWire`, `PlayerSkillsWire`, `ItemTemplateArgs`; deprecated shims in `outgoing_extra.rs`
- [x] `map_description.rs`: tile/map/move encoders take `&Codec`
- [x] `GameWorld.codec` + `enqueue_encoded`; `run_server` / `test_world` init
- [x] §3.5 rewire: `game_world`, `login_out`, `walk`, `container_ui`, `game_world_inventory`, `spawn_lifecycle`, `player_inventory_notifications`
- [x] Golden tests: `protocol_compat.rs` + `map_description.rs` via `Codec1098`
- [x] `cargo check --workspace`, `cargo test -p tfs-rust-net`

## Phase A3 — Transport capability gating (Track A) — done
C++ refs — 772: `gameserver/src/protocol.cpp` `XTEA_decrypt` (no checksum, `len-4`), `networkmessage.h`
`INITIAL_BUFFER_POSITION = 4`, `connection.cpp` (no `onConnect` challenge). 1098: repo-root
`src/protocol.cpp` `XTEA_decrypt` (4-byte Adler header, `len-6`), `networkmessage.h` IBP = 8,
`connection.cpp` checksum read.
- [x] A3.1 Plumb `ProtocolCaps` into `decrypt_xtea_game_body` / `encrypt_xtea_game_frame` (caps already on wire config).
- [x] A3.2 Gate Adler header read/write + buffer offset by `caps.adler_checksum` / `caps.initial_buffer_position` (cipher offset = `IBP - 4`).
- [x] A3.3 XTEA recv slack `-4` vs `-6` subsumed by the caps-driven cipher offset (772 = 0, 1098 = 4).
- [x] A3.4 Gate pre-login `0x1F` challenge send by `caps.prelogin_challenge`; only verify echo when sent (`Option<GameChallenge>`).
- [x] A3.5 Update callers: `server.rs` (game + login), packet-proxy `decrypt.rs`/`connection.rs`, tests.
- [x] Tests: XTEA frame encode→decode round-trip under both caps profiles; 772 no-checksum; cross-profile guard; 1098 unchanged.
- [x] Gate: `cargo check --workspace`, `cargo test -p tfs-rust-net -p tfs-rust-common` green (clippy errors are pre-existing baseline, unchanged by A3).

## Phase A4 — Login capability gating (772 / 1098) — done

Goal: login parse/encode branch on caps; DB gains account-number auth; 1098 byte-identical.
C++ refs — 772: `gameserver/src/protocolgame.cpp` `onRecvFirstMessage`, `protocollogin.cpp`
`onRecvFirstMessage`/`getCharacterList`/`disconnectClient`, `iologindata.cpp` `gameworldAuthentication`/
`loginserverAuthentication` (`accounts.id`). 1098: repo-root `src/protocollogin.cpp`, `protocolgame.cpp`.

- [x] A4.1 `LoginIdentity` enum (`AccountName(String)` 1098 | `AccountNumber(u32)` 772) in `game_first_packet.rs`; thread `&ProtocolCaps` into parse.
- [x] A4.2 Branch credential-block parse: 1098 session key (`acc\npass\ntoken\ntime` + char + challenge); 772 inline `[u8 gm][u32 acct][string char][string pass]` (game) / `[u32 acct][string pass]` (login). Split testable `parse_{game,login}_credentials`.
- [x] A4.3 DB `loginserver_authentication_by_number` / `gameworld_authentication_by_number` (`accounts.id`).
- [x] A4.4 `build_login_success`/`build_login_error` caps-gated: 772 = no `0x28`, per-char `name+server+u32 ip+u16 port`, `u16` premium days, `0x0A` error; 1098 byte-identical (legacy shim retained).
- [x] A4.5 `0x28` session-key send gated (772 omits); self-appear opcode already version-keyed (A2).
- [x] server.rs branches DB auth on `LoginIdentity`; packet-proxy threads 1098 caps.
- [x] Tests: 1098 encode/parse unchanged + new 772 credential/char-list units. `cargo check/clippy/test -p tfs-rust-net -p tfs-rust-common -p tfs-rust-db` green.
