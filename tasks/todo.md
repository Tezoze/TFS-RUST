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
