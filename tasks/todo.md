# Phase 7 — Globalevents

**Status:** done 2026-08-24.

Close the GlobalEvent half-state. Rust owns startup DB ops and the daily save clock. Drain pending GlobalEvents for startup / shutdown / record only. Ship optional `data/scripts/globalevents/record.lua`. Delete `data/globalevents/`.

## Plan

1. Write this file (parent).
2. `tfs-rust-db`: `startup_ops.rs` — truncate `players_online`, expire bans/wars, purge deleted players, sync towns; login/logout maintain `players_online`. Skip house auctions.
3. `tfs-rust-core`: `game_state.rs` + `server_save.rs` — wall-clock save from `config.lua`; Closed blocks new logins; reuse `flush_online_players_to_db`.
4. `tfs-rust-lua`: `global_events.rs` — drain `_pending_global_events`; dispatch startup/shutdown/record; warn (no dispatch) for `:time` / `:interval`.
5. Bind `Game.getPlayers`; allowlist `globalevents/record.lua`; persist `server_config.players_record`.
6. Delete `data/globalevents/`; drop XML test exception; update policy/README/plan.
7. Tests + `rtk cargo test` / clippy; `tasks/lessons.md`.

## Out of scope

House auctions, `Game.setGameState` / `saveServer` Lua bindings, `db`/`result` globals, Lua `onTime`/`onThink` dispatch.
