# Runtime Lua pack gaps (firstlogin /goto /killall /save)

**Status:** done.

Logs from 2026-08-24 live session.

## Plan

1. `player:addItem` must `push_item_userdata` so backpack 1987 is Container (`firstlogin.lua:15`). Hydrate the bag in `lua_script_player_add_item_full`.
2. `Creature(name)` — TFS `luaCreatureCreate` string arm via `get_creature_by_name` (`/goto Demon`).
3. `creature:addHealth` on monsters (TFS `luaCreatureAddHealth`); 0 HP → `apply_creature_death` (`/killall`).
4. Global `saveServer` / `Game.saveServer` queues `ServerSaveTick::FlushStay` (`luaSaveServer`).

## Verify

`rtk cargo test -p tfs-rust-lua --lib` — 170 passed
`rtk cargo test -p tfs-rust-core --lib add_health` — 3 passed
`rtk cargo test -p tfs-rust-core --lib add_item_full_backpack` — 1 passed
`rtk cargo run -p tfs-rust-lua --bin emit-lua-defs`
