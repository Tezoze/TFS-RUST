# Changelog

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
- Default `playerSpeed` is `"balanced"`. Strict linear speed is `playerSpeed = "772"`.
