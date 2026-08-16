# Action Lua API wiring — TYPE / TILEP / SPLASH / LOS — 2026-08-16

**Status:** done.

## Shipped
- **TYPE** — `item.type` / `getSubType` (`ScriptItemData.sub_type`). `create_bread.lua` flour+water.
- **TILEP** — `Tile.isPlayer` / `isMonster` / `isNpc` in `data/lib/core/tile.lua`. Ice pick on floor. Test uses Phase 2 `load_data_lib`.
- **SPLASH** — replace existing splash on `createItem`; **no** TFS ladder discard (772 `CreatePool`). Fluids show on ladders.
- **LOS** — Action `allowFarUse` → `map.throw_possible`; `CannotThrow`. Fishing through walls blocked.

## Deferred
G10 `Item:decay(id)` / SAY5 5-arg `say` with `music.lua` rewrite.
