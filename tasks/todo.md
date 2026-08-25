# House system

**Status:** complete.

Corpus: `houses.cc` monthly rent, 7-day grace, depot cash, town-depot eviction.
Pack surface: TFS `house.cpp` / `houses.xml` / `house_lists` / `tile_store` / Lua `House`.
Acquisition: MyAAC writes `bid`/`bid_end`/`highest_bidder`; server settles only.

## Done

1. `houses_xml.rs` + pipeline `house_file` load. Map walk attaches tiles/doors/beds.
2. `house.rs` → `house/` (access, registry, ownership, rent, auction, serialize, window, persist, tick).
3. `tile_store` load at boot; save on daily save + SIGINT shutdown.
4. `set_owner` eviction → town depot; kick occupants; login redirect to entry.
5. Daily tick: settle MyAAC auctions then collect rent (depot cash, 7-day grace).
6. `HouseStore` full metadata + `saveHouseInfo` upsert + auction candidate query.
7. Lua `House` API + `House(id)` / `Game.getHouses` + `setEditHouse`/`sendHouseWindow`.
8. Wire 0x97/0x8A house window (`0x97 | 0x00 | u32 | string`).
9. Tests + `tasks/lessons.md`.
10. House-door look: corpus NAMEDOOR clause (`house/look.rs`) — `You see a closed door. It belongs to house 'Name'. X owns this house.`

## Verify

`rtk cargo check --workspace`
`rtk cargo test -p tfs-rust-core house`
`rtk cargo test -p tfs-rust-content houses_xml`
`rtk cargo test -p tfs-rust-net house_window`
`rtk cargo test -p tfs-rust-lua --lib house`
`rtk cargo test -p tfs-rust-lua --lib lua_defs`
