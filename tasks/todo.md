# Talkactions Lua Phase 5–6

**Status:** complete.
**Source:** [talkactions-lua-implementation-plan.md](talkactions-lua-implementation-plan.md) Phases 5 and 6.

## Phase 5 — God spawn/admin

- `Game.createNpc` → slotless `lua_script_create_npc` (`spawn_lifecycle.rs`); returns `Npc` userdata.
- `npc:setMasterPos` → NPC `home_position` / optional radius.
- `Game.setGameState` bound (`GAME_STATE_NORMAL=2`, `CLOSED=3`, `SHUTDOWN=4`). Shutdown queues save+exit.
- `Game.startRaid` / `getReturnMessage` already wired.
- `Game.unlockAccount` / `unlockIp` always `true` (TFS login-attempt maps; no lock map yet). Ban rows stay `/unban` SQL.
- `getIPNumberFromString` (LSB = first octet).
- `Game.reload` slim: talkactions + scripts-interface; other types log+true.
- `refreshMap` no-op + log, return 0.
- `Game.getItemAttributeByName`; `setAttribute` accepts numeric strings.

## Phase 6 — Players / tutors leftovers

- Pack Lua already has `removeTotalMoney` and premium-day helpers over native money/bank/`premium_ends_at`.
- Native: `setSex`, `getParty` / Party share XP, `getSkull`, `canSeeCreature`, `getWorldUpTime`, `getSpeed`, `getAccountId`, `Game.getExperienceStage`.
- `configManager.getNumber` for `RATE_*` and `KILLS_*` (TVP `configKeys` indices).

## Verify

```
rtk cargo test -p tfs-rust-lua --lib
rtk cargo test -p tfs-rust-core --lib lua_script_create_npc
rtk cargo run -p tfs-rust-lua --bin emit-lua-defs -- --check
```
