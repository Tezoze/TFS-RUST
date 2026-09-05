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

## Party lifecycle — audit §1.2 (2026-08-29)

Wire client opcodes `0xA3`–`0xA8` to corpus `operate.cc:3919–4214`.

- [x] Extend `Party` (`invited`, `PartyShield`); fix `split_shared_experience` (even split, no TFS bonus)
- [x] `party.rs` — invite / revoke / join / pass leadership / leave / disband / share XP
- [x] `skulls.rs` — `player_get_party_mark`, `send_creature_shield_to_conn`
- [x] `game_loop.rs` — dispatch + timed-action whitelist
- [x] Logout forced leave; login `party_shield` from mark
- [x] Unit tests in `party.rs`

## NPC shop runtime — audit §1.3 (2026-09-05)

TFS shop-window pack surface (`0x79`–`0x7C`). 772 dialogue trading stays on `npc/host.rs`.

- [x] `shop.rs` — open/close, look, buy, sell, sale-list refresh
- [x] `Player.shop_owner` + `shop_items`; money + capacity on native buy
- [x] `game_loop.rs` dispatch; logout/remove close shop
- [x] Lua `openShopWindow` / `closeShopWindow` (`npc_shop.rs`)
- [x] Unit tests in `shop.rs`

## VIP runtime — audit §1.4 / step 3b (2026-09-05)

TFS pack surface `Game::playerAddVip` / `playerRemoveVip` / `playerEditVip`. List already loads at login.

- [x] `vip.rs` — add/remove/edit, `getMaxVIPEntries` (group / premium 100 / free 20, hard cap 200)
- [x] `game_loop.rs` dispatch `0xDC`–`0xDE`; `VipLookupFinished` for offline name resolve
- [x] `PlayerStore` INSERT/DELETE/UPDATE `account_viplist` (not on `savePlayer`)
- [x] Login/logout `notifyStatusChange`; era-correct `0xD2` / `0xD3` (`0xD4` logout on 772)
- [x] Unit tests in `vip.rs`; VIP codec goldens

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
