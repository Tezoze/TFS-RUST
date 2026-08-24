# Phase 8 — Data-pack Lua tests

**Status:** done 2026-08-24.

Make the Phase 8 checklist fail-closed in CI. No new gameplay. Loot is rolled once at spawn; death moves items; Lua mutates only in `onSpawn`.

## Plan

1. Write this file (parent).
2. New `crates/tfs-rust-core/src/data_pack_lua_tests.rs` — 8.1 ItemId identity spawn→corpse, 8.2 rarity attribute survives death, 8.3 summon corpse empty, 8.4 native loot without Lua, 8.7 AoL + PlayerDeath Lua zero item delta, 8.9 V772/V1098 twins. Do not grow `game_world.rs` / `lua_event_dispatcher.rs` / `monster_inventory.rs`. Tighten `test_e6_corpse_contains_spawn_loot` to identity or drop the weaker assert.
3. Lua: 8.4 missing `eventcallbacks/` dir; 8.8 `call_monster_on_spawned` sentinel when unregistered. `scripts/check_data_pack_policy.sh`: drop stale `globalevents.xml` allow.
4. `rtk cargo test` / clippy on touched crates; mark Phase 8 done in plan + `docs/DATA_PACK_LUA.md`; `tasks/lessons.md`.

## Out of scope

Enabling `rarity.lua`, `/reload`, `Game.createItem` onDeath panic tests, house auctions, `db` global, re-deriving movements XML.
