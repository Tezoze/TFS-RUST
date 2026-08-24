# Talkactions Lua — bind pack primitives

**Status:** in progress (phase 1–2).

Live 2026-08-24: `/deathlist` (`db` nil), `/ghost` (`setGhostMode` nil), `/info` `/mccheck` (`getIp` nil). `/kick` vs another access player is correct.

## Plan

0. Write `tasks/talkactions-lua-implementation-plan.md`.
1. `getIp`, `setGhostMode`, `popupFYI`; session IPv4 on `Player` at login. **done**
2. `player:remove` / `creature:remove` as kick (forced logout). **done**
3. `lua_database.rs`: `db` / `result` over sqlx via oneshot (game thread waits; IO runs SQL).
4. `userdata/house.rs` + `Game.getHouses`.
5. God leftovers: `createNpc`, `setGameState`, raid/unlock/reload-slim.
6. Player/tutor leftovers: money, premium, party, `configManager.getNumber`.
7. `emit-lua-defs`, tests, `tasks/lessons.md`.

## Verify

`rtk cargo test -p tfs-rust-lua --lib`
`rtk cargo test -p tfs-rust-core --lib`
`rtk cargo run -p tfs-rust-lua --bin emit-lua-defs`
