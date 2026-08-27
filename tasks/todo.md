# Talkactions Lua Phase 4 — House

**Status:** complete.
**Source:** [talkactions-lua-implementation-plan.md](talkactions-lua-implementation-plan.md) Phase 4.

House runtime was already in place (`HouseManager`, XML, OTBM tiles, `setOwnerGuid`). This phase finished the remaining Lua surface.

## Done

- Existing: `House(id)`, `Game.getHouses()`, `tile:getHouse()`, `player:getHouse()`, reads + `setOwnerGuid` / access lists / `kickPlayer` / `save`.
- **`isGuildHall`:** XML `guildhall=` when present; 772 names inferred (`Guildhall`, `, Guild`, leading `Guild`). Stored on `HouseXmlEntry` / `House` / `ScriptHouseData`.
- **`getTileCount`:** OTBM `tiles.len()` when attached, else XML `size`.
- **`startTrade`:** TFS `luaHouseStartTrade` `RETURNVALUE_*` checks; no transfer item (player trade not ported) → `YOUCANNOTTRADETHISHOUSE` (67). Always an integer for `!sellhouse`.
- `emit-lua-defs` includes `House:startTrade`.

## Verify

```
rtk cargo test -p tfs-rust-lua --lib userdata::house
rtk cargo test -p tfs-rust-content --lib houses_xml
rtk cargo test -p tfs-rust-lua --lib lua_defs
rtk cargo test -p tfs-rust-core --lib house
rtk cargo run -p tfs-rust-lua --bin emit-lua-defs -- --check
```

`/gotohouse` and `/owner` are unblocked. Paid `!buyhouse` still needs Phase 6 `removeTotalMoney`. `!sellhouse` cancels with “You can not trade this house.” until P2P trade exists.
