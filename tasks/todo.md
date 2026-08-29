# Native Handler Migration

**Status:** complete.

## Native data/lib API (2026-08-29)

Ported stateful `data/lib/core` helpers to native Rust + Lua-callable bindings.

- [x] `register_data_lib_native` after `load_data_lib` (`run_server.rs`, tests)
- [x] Phase 1a — `game_map_helpers.rs`, `GameWorld.global_storage`, `Game.*` natives
- [x] Phase 1b — native `Combat:getPositions` + `resolve_combat_area_context`
- [x] Phase 1c — `player_money_lib.rs`, `Player.removeTotalMoney` / `canCarryMoney`
- [x] Phase 2 — `creature_lib.rs`, summon/outfit/path/PZ natives on `CreatureRef`
- [x] Phase 2 misc — `Player.getDepotItems`, `Player.getClosestFreePosition` override
- [x] Phase 3 — item look, `Position:isInRange`, `ItemType.usesSlot`, `Vocation.getBase`, `Party.broadcastPartyLoot`, `Tile.relocateTo`
- [x] Pack Lua slimmed/stubbed (`game.lua`, `combat.lua`, `creature.lua`, `item.lua`, …)

**Stay Lua:** `constants.lua`, `storages.lua`, `create_functions.lua`, `container.createLootItem` error stub.

## Phase 2 — EventCallback dispatch (ship first)
- [x] Rust-side `has_event_callback` bitset + direct RegistryKey dispatch
- [x] Sync from `EventCallbackData` at end of `load_scripts_interface`

## Phase 1 — MoveEvent aid native path (3000–3123)
- [x] `aid_move_events.rs` + `aid_move_compile.rs` + dispatch hooks + boot log

## Phase 1c — Native moveitem policy
- [x] `player_move_policy.rs` — quest aid, candelabrum, blocking tile, trap (no VM per move)
- [x] `spell_combat_compile.rs` — boot parse Combat specs from spell/rune scripts
- [x] `native_spell_combat.rs` — skip `onCastSpell` VM; call `combat_execute_from_lua` directly
- [x] `fire_on_cast_spell` / `fire_on_cast_rune` try native first; boot log `native_spell_combats`
