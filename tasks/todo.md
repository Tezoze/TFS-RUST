# Movements Lua APIs — M3 setTown + getMaster — 2026-08-16

**Status:** M3 done. Full audit: [movements-plan.md](movements-plan.md).

## M3 shipped
- **M3a** `Player:setTown(Town)` — Town userdata only; `true` / `false` / `nil` like TFS; in-memory `town_id` only (no teleport).
- **M3b** `Creature:getMaster()` — live summoner `CreatureRef` or `nil`.

Then **M4** (772 trap/field script pass).
