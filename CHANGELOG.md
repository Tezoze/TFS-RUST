# Changelog

## Unreleased

Playable 8.0 shard, not tagged yet. Login port 7171, game port 7172, `clientVersion = 800`. Mechanics stay the 772 corpus.

### Classic 8.0

Protocol 800 uses the same 772 mechanics corpus, with its own wire codec and `data/formulas/800.lua`.

- Hotkey object use.
- Step timing follows the client walk cycle. 8.0 walk speed uses the linear model, so that era can be retuned on its own. The scheduler beat stays 50 ms.
- The 8.0 fluid palette, and walk flags taken from `items.srv`.
- One item id across the catalog, the map, and scripts. The 8.0 catalog is extended from `items.srv`. Load path is the versioned catalogs (`data/items/772`, `800`, `1098`). The old root item dumps are gone.
- A house load still accepts a pre-cutover id when it is the old id of that map item. GM look prints that single id.
- Rune charges report the drawn count. Attack runes print exhaustion.
- Stamina, outfit scripts, a death bag, a quest toast, and the post-8.0 spell range.
- `playerSpeed = "classic"` selects the linear curve for both 772 and 8.0. The old 772 label still maps to it.

### World

- Changed non-house tiles persist in `data/world/forgotten-live.bin` and are restored on boot. Ctrl+C refreshes those tiles before the write. The checked-in overlay is two tiles: mummy remains holding two worms at 33005, 32397, 11, and a dead spider with a pool at 32648, 32082, 12. The earlier persist test (empty vials, a rope, and a letter) is no longer in that file.
- Sleeping in a bed saves the player and closes the client.
- Logout closes the socket in the same call on every client version.

### Monsters

- A monster that already has a chase target keeps it when another creature steps inside its view. The walk armed by idle waits until the next beat.

### Combat

- A close-chase step does not strike while the attack is still exhausted. The strike waits until that delay ends.

### NPCs

- Shared shop and guard lists live in `data/npc/catalogs/`. Scripts splice them with `NpcAppendRules` instead of copying each list into the script. The old `.npc` / `.ndb` archive is no longer in the tree.

### Release tree and CI

- `tasks/` and the audit notes stay on disk and out of git. The compiling and Docker guides stay tracked. Unused NPC fixture traces and the local helper scripts are not published. Start the server with `cargo run --bin tfs-rust`.
- CI no longer runs the criterion benchmark or the `tfs-rust-sim` harness. Item tests load the tracked 772 `items.ron` catalog. LuaLS knows the injected tool globals (`actionIds`, `corpseIds`, `ropeSpots`, `Fields`) and the return types of `Game.getPlayers`, `Game.getHouses`, and the SQL result queries.

## 1.0.0 — 2026-09-26

Playable 7.72 shard. Login port 7171, game port 7172, `clientVersion = 772`.

This release ships:

- 772 wire and native mechanics (walk, combat, monsters, death, loot, houses, trade, party, shop, VIP, mail). No-profession fed mana regenerates 2 per tick, the same amount as every other vocation.
- The TFS `data/` script surface on top of that native core.
- A release binary (`cargo build --release --bin tfs-rust`) and a GHCR image on tag `v1.0.0`.

Known limitations, shipped as-is:

- Protocol 10.98 and later-era features (market, mounts, stamina gameplay) stay behind version gates.
- House auctions (`TransferHouses` / `StartAuctions`) stay on the AAC website. In-game house sale is `!sellhouse` and player trade.
- Some spell formulas still follow the data pack: healing does not clear paralyze on its own, some area spells use the pack matrices, and berserk and mana-fluid rolls differ from the 772 corpus.
- Channel Lua hooks (`canJoin`, `onJoin`, `onSpeak`) are stubbed. `LookInBattleList`, `JoinAggression`, and `CloseNpcChannel` are parsed and dropped.
- Default `playerSpeed` is `"balanced"`. Strict linear speed is `playerSpeed = "classic"`.
