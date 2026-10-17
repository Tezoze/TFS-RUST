# Changelog

## Unreleased

Playable 8.0 shard, not tagged yet. Login port 7171, game port 7172, `clientVersion = 800`. Mechanics stay the 772 corpus.

### Classic 8.0

Protocol 800 uses the same 772 mechanics corpus, with its own wire codec and `data/formulas/800.lua`.

- 2026-09-26 — Hotkey object use.
- 2026-09-26 — Step timing follows the client walk cycle. 8.0 walk speed uses the linear model, so that era can be retuned on its own. The scheduler beat stays 50 ms.
- 2026-09-26 — One item id across the catalog, the map, and scripts. The 8.0 catalog is extended from `items.srv`. Load path is the versioned catalogs (`data/items/772`, `800`, `1098`). The old root item dumps are gone.
- 2026-09-26 — A house load still accepts a pre-cutover id when it is the old id of that map item. GM look prints that single id.
- 2026-09-26 — Rune charges report the drawn count. Attack runes print exhaustion.
- 2026-09-27 — The 8.0 fluid palette, and walk flags taken from `items.srv`.
- 2026-09-27 — Stamina, outfit scripts, a death bag, a quest toast, and the post-8.0 spell range.
- 2026-09-27 — `playerSpeed = "classic"` selects the linear curve for both 772 and 8.0. The old 772 label still maps to it.
- 2026-10-11 — A same-floor diagonal waits three waypoints before the next step. A floor change stays one waypoint. The 10.98 log curve uses that same scale.
- 2026-10-11 — Letters and labels show the written date from client 7.9 on. Anonymous channel speech hides the speaker. Tutors can report bugs, and an 8.0 gamemaster login sends the staff viewport bytes.
- 2026-10-11 — `maxPlayers` queues a full server. 7.72 and 8.0 clients are told their place on the list. Gamemasters and the always-login flag skip the queue. Leave the key unset and the server stays open.
- 2026-10-11 — 7.72 anonymous channel speech sends an empty name. A private message with no speaker is not sent. A 7.72 gamemaster login sends the banishment-rights packet after the character appears. 8.0 still sends the staff viewport bytes on the appear packet.
- 2026-10-11 — Stopping during a push wait cancels that push when the wait ends. The item or creature stays put, and the client gets a position snapback.
- 2026-10-11 — The rule-violation channel is open to gamemasters. Reports, answers, and cancels use the 8.0 channel packets. 7.72 still uses the gamemaster request.

### World

- 2026-09-27 — Changed non-house tiles persist in `data/world/forgotten-live.bin` and are restored on boot. Ctrl+C refreshes those tiles before the write. The checked-in overlay is two tiles: mummy remains holding two worms at 33005, 32397, 11, and a dead spider with a pool at 32648, 32082, 12. The earlier persist test (empty vials, a rope, and a letter) is no longer in that file.
- 2026-09-27 — Sleeping in a bed saves the player and closes the client.
- 2026-09-27 — Logout closes the socket in the same call on every client version.
- 2026-10-11 — The health and mana granted on wake come from the food and offline time, not from how long the character was in the bed.
- 2026-10-11 — The Rookgaard level-2 bridge drops a level 1 player on the same x, one tile south, floor 7.

### Monsters

- 2026-09-27 — A monster that already has a chase target keeps it when another creature steps inside its view. The walk armed by idle waits until the next beat.
- 2026-10-11 — `monsterSpells.cadence` in the formula file is `machineGun` by default: every idle still rolls every spell. `round` is one shared pass per interval (adjacent 2000 ms, at range or fleeing 1000 ms, a melee monster still chasing 4000 ms). Several spells can still land on that pass.
- 2026-10-11 — The pack includes the corpus races butterfly and human. Colored butterflies stay separate races.

### Combat

- 2026-09-27 — A close-chase step does not strike while the attack is still exhausted. The strike waits until that delay ends.
- 2026-09-27 — A healing spell clears paralyze on its own, including the intense and ultimate healing runes.
- 2026-09-27 — Poison storm covers the full disc. Berserk costs four times the caster's level, and its damage is one roll scaled by level and magic level. Mana fluid restores 50–150 mana and life fluid restores 25–75 hit points, both on the server roll.
- 2026-10-11 — Drinking a life fluid clears paralyze. Mana fluid and life fluid also start the 1 second spell cooldown and show the blue effect.
- 2026-10-11 — A hit that mana shield fully absorbs still blocks logout, including field damage with no attacker. Mana drain on a monster clears invisibility and wakes it.
- 2026-10-11 — Food regeneration uses the active vocation. A promoted character on a free account regenerates as the base vocation.
- 2026-10-11 — A skill whose growth factor is below 1.05 costs the base number of tries for each level. Monster fist uses that path.
- 2026-10-11 — Sleeping in a house bed writes the sleeper's name on the bed. At login the corpus formula applies: mana is `min(food / 3, offline / 15)`, health is a quarter of that, food spends three times that, and soul gains `offline / 900` up to the soul maximum.
- 2026-10-11 — Haste, strong haste, paralyze, mana shield, invisibility, and light keep the corpus durations. Formula files no longer set those timers.
- 2026-10-11 — Cancel invisibility leaves the caster's tile, protection zones, and peaceful creatures on a no-PvP world hidden. It only reveals someone who is actually invisible.
- 2026-10-11 — Drinking slime applies a poison of strength 200. A stronger poison already on the player stays.
- 2026-10-17 — A spell is blocked only by the spell wait. A weapon swing waits until both the attack wait and the spell wait are over. Using two objects has its own wait. Walking does not use these waits.

### Items

- 2026-10-17 — One item move checks a map tile, an inventory slot, and a container separately. A chest still counts as having room when the item comes from a container or an inventory slot. A move from a tile onto a container does not use that chest exception.

### NPCs

- 2026-09-27 — Shared shop and guard lists live in `data/npc/catalogs/`. Scripts splice them with `NpcAppendRules` instead of copying each list into the script. The old `.npc` / `.ndb` archive is no longer in the tree.
- 2026-10-11 — An NPC that summons a monster is that monster's master.

### Chat

- 2026-10-11 — Channel scripts run. `canJoin` can refuse a join. `onSpeak` can drop a line or change its color. `onJoin` and `onLeave` fire when a player enters or leaves.

### Release tree and CI

- 2026-10-17 — An NPC shop catalog uses a derived `Default`. Rust 1.99 Clippy rejects the hand-written impl, which was failing CI.
- 2026-09-26 — `tasks/` and the audit notes stay on disk and out of git. The compiling and Docker guides stay tracked. Unused NPC fixture traces and the local helper scripts are not published. Start the server with `cargo run --bin tfs-rust`.
- 2026-09-27 — CI no longer runs the criterion benchmark or the `tfs-rust-sim` harness. Item tests load the tracked 772 `items.ron` catalog. LuaLS knows the injected tool globals (`actionIds`, `corpseIds`, `ropeSpots`, `Fields`) and the return types of `Game.getPlayers`, `Game.getHouses`, and the SQL result queries.

## 1.0.0 — 2026-09-26

Playable 7.72 shard. Login port 7171, game port 7172, `clientVersion = 772`.

This release ships:

- 772 wire and native mechanics (walk, combat, monsters, death, loot, houses, trade, party, shop, VIP, mail). No-profession fed mana regenerates 2 per tick, the same amount as every other vocation.
- The TFS `data/` script surface on top of that native core.
- A release binary (`cargo build --release --bin tfs-rust`) and a GHCR image on tag `v1.0.0`.

Known limitations, shipped as-is:

- Protocol 10.98 and later-era features (market, mounts, stamina gameplay) stay behind version gates.
- House auctions (`TransferHouses` / `StartAuctions`) stay on the AAC website. In-game house sale is `!sellhouse` and player trade.
- Channel Lua hooks (`canJoin`, `onJoin`, `onSpeak`) are stubbed. `LookInBattleList`, `JoinAggression`, and `CloseNpcChannel` are parsed and dropped.
- Default `playerSpeed` is `"balanced"`. Strict linear speed is `playerSpeed = "classic"`.
