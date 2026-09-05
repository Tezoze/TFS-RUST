# 772 Parity Gap Audit — Remaining Work

**Date:** August 2026
**Scope:** Mechanics/corpus parity only. Wire codec, query manager, login, and DB infrastructure were explicitly out of scope for this pass.
**Method:** Function-level sweep of the decompile corpus (`reference/cipsoft-772/tibia-game-master/src/`) against `crates/` and `data/`, one auditor per corpus system cluster.

## Reading this document

Status is judged **by observable behavior, not by naming**. Large parts of the port intentionally wear the TFS pack surface (`ConditionDamage`, `Combat:execute`, `Action()`/`MoveEvent()`, monster Lua defs) while reproducing corpus outcomes. A behavior implemented under a TFS-shaped name is **DONE**, not a gap.

Likewise, several corpus subsystems are deliberately **not** ported as engines — the `moveuse.dat` rule VM being the largest. Those are recorded in [Intentional deviations](#intentional-deviations) rather than counted against completeness.

## Completeness snapshot

| System | Corpus files | Estimate | State |
|---|---|---|---|
| Player / skills / combat | `crplayer.cc`, `crskill.cc`, `crcombat.cc`, `crmain.cc` | ~85% | Strongest area. Remaining items are metadata and fidelity, not mechanics. |
| Monster / NPC AI | `crnonpl.cc`, `cract.cc` | ~90% | Idle-stimulus engine, spawn, chase, casting, loot, NPC dialogue, home/LifeEnd despawn all live. |
| Magic / spells | `magic.cc` | ~85% | Pre-cast rune ML / exhaust / PZ and spoken premium done. Remaining: heal-paralyze, AoE rings, berserk, mana fluid. |
| Map | `map.cc`, `info.cc` | ~78% | Stacking, flags, throw LOS, decay cron done. Live sector refresh missing. |
| Houses | `houses.cc` | ~75% | Ownership, rent, lists, doors, eviction, **in-game sell via trade** done. Policy evictions and transfer missing. |
| Move / use | `moveuse.cc`, `objects.cc` | ~70% | Typed handlers, doors, fields, tools done. Mail missing; script numerics drift. |
| Chat / channels | `operate.cc`, `crplayer.cc` | ~88% | Say 7×5, yell 30×30, RecordTalk, trade-offer gate, PM cap, guild look/filter done. Lua channel hooks and a few packets still stubbed. |
| Player operations | `operate.cc` | ~82% | Trade (1.1), party (1.2), NPC shop (1.3), and VIP (1.4) dispatched. |
| Info / script | `info.cc`, `script.cc`, `config.cc` | ~55% | Mostly replaced by OTBM + `config.lua` + Lua by design. |

Note that `communication.cc` turned out to contain no chat mechanics at all — it is socket, login, and waiting-list only (`communication.hh:24-64`). The 772 talk and channel behavior lives in `operate.cc` (`Talk`, `TChannel`, `OpenChannel`) and `crplayer.cc` (`RecordTalk`, `LeaveAllChannels`). Cite those instead.

---

## Tier 1 — Subsystems that are absent

These are whole features where the client sends a request and nothing happens. They dominate the remaining work.

VIP, trade, party, and shop packets parse in `crates/tfs-rust-net/src/game_parse.rs`. **§1.1 trade, §1.2 party, §1.3 shop, and §1.4 VIP are complete** — see below.

### 1.1 Player-to-player trade — **DONE**

**Corpus:** `TCreature::ToDoTrade` / `Trade` (`cract.cc:653-725`, `:1202-1256`), `TPlayer::InspectTrade` / `AcceptTrade` / `RejectTrade` (`crplayer.cc:811-1000`), invalidation via `NotifyTrades` (`operate.cc:990-1023`, called from move paths). Wire out via TVP `sendTradeItemRequest` / `sendCloseTrade` (`gameserver/src/protocolgame.cpp`).

**Rust (August 2026):**

- Focused module [`crates/tfs-rust-core/src/trade.rs`](../crates/tfs-rust-core/src/trade.rs) — `TradeRegistry` / `TradeSide` on `GameWorld`, not extra `Player` fields.
- Packets `0x7D`–`0x80` dispatched in [`game_loop.rs`](../crates/tfs-rust-core/src/game_loop.rs); `RequestTrade` queues `TDTrade` via ToDo (walk-to-reach prepends `Go`); look/accept/reject immediate.
- Wire: `ProtocolCodec::encode_trade_item_request` / `encode_close_trade` (`0x7D`/`0x7E`/`0x7F`); golden bytes in [`protocol_compat.rs`](../crates/tfs-rust-net/tests/protocol_compat.rs) v772.
- `NotifyTrades` on item move (`game_world_item_move.rs`), inventory update/remove, walk out of range, logout/takeover/remove.
- Auto-stack skip via `player_trade_item` in `query_add.rs`.
- **House pack surface:** `house:startTrade` → `LuaMutation::HouseStartTrade`; dual `ITEM_DOCUMENT_RO` counter-offer; `house_set_owner` on dual accept — unblocks [`!sellhouse`](../data/scripts/talkactions/players/sellhouse.lua).

**Tests:** `cargo test -p tfs-rust-core --lib trade` (9 tests); protocol goldens; house cancel-code tests.

**Corpus notes (lesson 411):** partner-trading string `"This person is already trading."`; cancel always `"Trade cancelled."`; reject is asymmetric; max 100 nested objects; Chebyshev ≤2 + LOS.

### 1.2 Party lifecycle — **DONE**

**Corpus:** `InviteToParty` / `RevokeInvitation` / `JoinParty` / `PassLeadership` / `LeaveParty` / `DisbandParty` and `IsInvitedToParty` / `GetParty` (`operate.hh:189-196`, bodies at `operate.cc:3919-4214`). `TParty` holds leader, member vector, and invited-player vector (`operate.hh:68-82`).

**Rust (August 2026):**

- Focused module [`crates/tfs-rust-core/src/party.rs`](../crates/tfs-rust-core/src/party.rs) — `Party` with leader, members, invited; `PartyShield` marks.
- Packets `0xA3`–`0xA8` dispatched in [`game_loop.rs`](../crates/tfs-rust-core/src/game_loop.rs): invite / join / revoke / pass leadership / leave / share-XP toggle.
- Logout forced leave; invite tracking; skull/shield broadcast via `player_get_party_mark` / `send_party_creature_updates`.
- `split_shared_experience` is even divide only (no TFS party bonus). Same-party XP skip in `death.rs` unchanged.

**Tests:** `cargo test -p tfs-rust-core --lib party`.

### 1.3 NPC shop runtime — **DONE**

**Pack surface:** TFS `Game::playerPurchaseItem` / `playerSellItem` / `playerCloseShop` / `playerLookInShop` (`game.cpp`); `Player::openShopWindow` / `updateSaleShopList` (`player.cpp`); Lua `openShopWindow` / `closeShopWindow` (`npc.cpp`). 772 corpus vendors stay dialogue `create`/`delete`/`createmoney` — the shop window is a TFS pack/UI obligation.

**Rust (September 2026):**

- Focused module [`crates/tfs-rust-core/src/shop.rs`](../crates/tfs-rust-core/src/shop.rs) — `ActiveShopItem` catalog on `Player` (`shop_owner` + `shop_items`), not a world registry.
- Packets `0x79`–`0x7C` dispatched in [`game_loop.rs`](../crates/tfs-rust-core/src/game_loop.rs); look / buy / sell / close.
- Wire: `send_shop` / `send_sale_item_list` / `send_close_shop` (`0x7A` / `0x7B` / `0x7C`).
- Native buy: money (`player_remove_total_money`) + capacity; native sell: `player_remove_item_of_type` + `player_create_money`. Lua buy/sell callbacks run when `openShopWindow` registered them.
- `updateSaleShopList` on inventory add/remove; close shop on logout / creature remove.
- Lua: [`crates/tfs-rust-lua/src/npc_shop.rs`](../crates/tfs-rust-lua/src/npc_shop.rs) `openShopWindow` / `closeShopWindow`; `LuaMutation::OpenShopWindow`.

**Tests:** `cargo test -p tfs-rust-core --lib shop`.

### 1.4 VIP runtime — **DONE**

**Pack surface:** TFS `Game::playerAddVip` / `playerRemoveVip` / `playerEditVip` (`game.cpp`); `Player::addVIP` / `removeVIP` / `getMaxVIPEntries` / `notifyStatusChange` (`player.cpp`); `IOLoginData::{add,remove,edit}VIPEntry` (`iologindata.cpp`). List is per-account (`account_viplist`).

**Rust (September 2026):**

- Focused module [`crates/tfs-rust-core/src/vip.rs`](../crates/tfs-rust-core/src/vip.rs) — mutate `Player.vip_list`, not a world registry.
- Packets `0xDC`–`0xDE` dispatched in [`game_loop.rs`](../crates/tfs-rust-core/src/game_loop.rs); offline add via `GameCommand::VipLookupFinished`.
- Wire: `ProtocolCodec::encode_vip_entry` / `encode_vip_status` — 772 `0xD2`/`0xD3`/`0xD4`; 1098 full `0xD2` + `0xD3` status byte. `VipEdit` stays 1098-only incoming.
- Immediate SQL persist (not `savePlayer`); `getMaxVIPEntries` uses `groups.max_vip_entries` or premium 100 / free 20, hard cap 200.
- Login/logout `notifyStatusChange` to watchers.

**Tests:** `cargo test -p tfs-rust-core --lib vip`; protocol goldens `vip_entry_and_status_*`.

### 1.5 Mail

**Corpus:** `SendMail` / `SendMails` (`moveuse.cc:712-919`) — parses addressee and town from the letter text, delivers to the recipient's depot when online, queues when offline, and stamps the letter on send.

**Rust:** no equivalent found in `crates/`. Mailboxes exist in the map content and currently do nothing.

### 1.6 Live sector refresh

**Corpus:** `SectorRefreshable` / `RefreshSector` / `RefreshMap` / `RefreshCylinders` / `ApplyPatch` / `ApplyPatches` and `ProcessCronSystem` (`operate.hh:158-165`). `RefreshSector` (`map.cc:1307-1350`) tests the sector's `MapFlags & 0x01` (`map.cc:1320`), strips non-creature objects, and reloads from a patch stream.

**Rust:** the refresh flag is read from OTBM at load but never acted on at runtime; `Game.refreshMap` returns 0 and logs (lesson 411). The decay cron half of `ProcessCronSystem` **is** implemented (`game_world_tick.rs:91-99` + `decay_apply.rs`); only the sector-reload half is missing.

---

## Tier 2 — Behavioral gaps inside shipped systems

### Magic / spells

- **Rune pre-cast gates — DONE (Step 4, September 2026).** `player_cast_rune` now runs `CheckRuneLevel` (`rune_magic_level_ok`), `EarliestSpellTime`, and aggressive PZ before `fire_on_cast_rune`; fail does not consume. Helpers in [`spell.rs`](../crates/tfs-rust-core/src/spell.rs). Cancel text is corpus `"Your magic level is too low."`
- **Premium spells — DONE (Step 4).** `player_say_spell` reads `InstantSpellDef.is_premium` (`CheckAccount`); rune *use* still does not (corpus `UseMagicItem` never calls it).
- **Rune stacked-tile targeting — DONE (Step 4).** `prefer_rune_tile_target` + `UseWithCreature` `ActionObjectRef.creature_id` seed. Aggressive last non-self; heal prefers self. Generic use-with still takes `creatures.first()`.
- **Healing does not clear paralyze natively.** `THealingImpact` and `Heal` reset `SKILL_GO_STRENGTH` when the delta is negative (`magic.cc:203-205`, `:2113-2115`). Rust relies on individual scripts setting `COMBAT_PARAM_DISPEL`.
- **AoE radii use TFS matrices where the corpus uses rings.** Ultimate explosion is r=6 in the corpus (`magic.cc:3485-3487`) but `AREA_CIRCLE5X5` in `ultimate_explosion.lua:7` — note `AREA_CIRCLE6X6` already exists at `areas.lua:177`. Poison storm is r=8 (`magic.cc:3536-3539`) against `AREA_CIRCLE5X5`. Cancel invisibility is r=4 skipping origin (`magic.cc:2353-2450`) against `AREA_CIRCLE3X3`.
- **Berserk uses a different formula path.** Corpus case 80 is `(Level * ComputeDamage(...)) / 25` with mana `Level*4` (`magic.cc:3557-3562`); `berserk.lua:10` routes through `computeSkillDamage`.
- **Mana fluid roll differs.** `DrinkPotion` uses `ComputeDamage(NULL, 0, 100, 50)` (`magic.cc:4328-4333`); `fluids.lua:61-62` uses `math.random(50,150)`.

### Player operations and chat

- **Yell / say / whisper ranges — DONE (Step 5).** `Talk` post-filter `|dx|≤7 && |dy|≤5` same Z; yell `|dx|≤30 && |dy|≤30` with surface-only multifloor (`operate.cc:2357-2392`). Helpers in [`chat_talk.rs`](../crates/tfs-rust-core/src/chat_talk.rs). Whisper still garbles to `"pspsps"` outside Chebyshev 1.
- **Flood mute — DONE (Step 5).** Live model is corpus `RecordTalk` (2.5 s `ServerMilliseconds` window, trip when `TalkBufferFullTime > now + 7500`, mute `n²×5` rounds). TFS `maxMessageBuffer` / `5n²` is no longer applied. Pack `CONDITION_MUTED` still extends `player_is_muted`.
- **Trade-channel rate limit — DONE (Step 5).** `EarliestTradeChannelRound + 120` on pack Trade **id 6** (`trade.lua`), not corpus enum 5 (pack RL-Chat). Cancel: `"You may only place one offer in two minutes."`
- **Private-message spam cap — DONE (Step 5).** `RecordMessage` 20 slots / 600-round age; `"You have addressed too many players. You are muted for N second(s)."`
- **Guild look + channel filter — DONE (Step 5).** Look appends rank/`a member` + `of the <guild>` + optional nick. Guild-channel fan-out requires matching `guild_name` (`operate.cc:2445-2448`). Login JOINs `guilds` / `guild_ranks`.
- **Cancel texts — DONE (Step 5).** Premium private-channel create → `YouNeedPremiumAccount`; invite/exclude corpus info strings; `EditText` `len >= max` → `NotEnoughRoom`; `UseWithCreature` OOR → `DestinationOutOfReach`.
- **`LookInBattleList`, `JoinAggression`, and `CloseNpcChannel` are unhandled** — parsed, then dropped.
- **Lua channel hooks are stubbed.** `canJoin` / `onJoin` / `onSpeak` from `data/scripts/chatchannels/*.lua` are not wired (`game_world_chat.rs`). Trade's 2-minute Lua `onSpeak` is replaced by the native gate above; advertising/level-1 still need the hooks.

### Monster / NPC AI

- **Monsters outside their monsterhome — DONE (Step 6).** `IdleStimulus` calls `monsterhome_in_range` and `remove_creature` when false (`crnonpl.cc:2408-2414`). `home_radius <= 0` ≡ `Home == 0` (in range); `|dz|<=2` hardcoded. No ATTACKING exemption. MovePossible still skips the leash while chasing.
- **`LifeEndRound` — DONE (Step 6).** Drained at the top of `IdleStimulus` (`crnonpl.cc:2352-2356`) via `remove_creature`. Raid tick only *sets* the field at spawn.
- **No explicit `DistanceFighting` race flag.** The corpus reads it from `RaceData` (`crmain.cc:1253`, `:1498`) and branches at `crnonpl.cc:2837-2868`. Rust infers the distance branch from `target_distance > 1 && ThrowPossible` (`monster_ai.rs:217-226`). This currently produces correct results for the shipped pack, but it is a data-shape mismatch waiting to bite.
- **Four NPC behaviour actions are unimplemented:** `Bless` (7 call sites), `Town` (9), `String` assignment (595), `Promote` (4) — see `tasks/npc-corpus-inventory.md:85-88`.
- **NPC `Summon()` does not bind a master.** `npc/host.rs:134-144` creates a detached monster.

### Move / use

- **`MOVEMENTEVENT` on item cylinder transfer has no hook.** `moveuse.cc:2263-2287` fires when a flagged item moves between containers; the corpus uses it for quest items in chests. No equivalent in the item-move path.
- **`UseChangeObject` UNLAY shuffle is not replicated.** When a transform target is `UNLAY`, the corpus relocates stack objects to an adjacent passable tile (`moveuse.cc:2184-2204`) — distinct from `ClearField`, which *is* ported (`clear_field.rs:30+`). Currently only doors get the treatment (`doors.rs:164`).
- **`UseAnnouncer` cases 1 and 3 are missing** — full in-world date string (`moveuse.cc:1891-1898`) and the blessings list from quest values 101–105 (`:1909-1944`). Case 2 (time) and case 4 (spellbook) are done.
- **Level/quest door denial text is hardcoded.** The corpus reads the item's info string via `GetInfo(Door)` (`moveuse.cc:2075`, `:2111`); `doors.rs:196`, `:218` use fixed strings, which loses map-specific messages.

### Map / houses

- **Splash and pool items are on the wrong layer.** They belong on BOTTOM (`CreatePool` scans BOTTOM, `operate.cc:2585+`) but OTB `FLAG_ALWAYSONTOP` routes them into `top_items`, so they render above creatures. Already written up in `docs/772_SPLASH_LAYER_MISMATCH.md`; content-side guards currently paper over it.
- **Elevation climb defects remain.** Four are enumerated in `docs/772_ELEVATION_WALK_PARITY.md` §4/§5 against `walk/walk_tile.rs`.
- **House policy evictions are absent:** `EvictFreeAccounts` (`houses.cc:1139+`), `EvictDeletedCharacters` (`:1173+`), `EvictExGuildLeaders` (`:1199+`).
- **`TransferHouses` (`houses.cc:1029+`) and `StartAuctions` (`houses.cc:1334+`) are not ported.** Auction *settlement* is (`house/auction.rs:18-36`), on the assumption MyAAC writes the bid columns — worth confirming that schema matches the `FinishAuctions` payment check.
- **Corpus `MayOpenDoor` parses access rules from the door's own text** (`houses.cc:562-619`). Rust uses DB `door_lists` (`house/mod.rs:210-224`), which is the TFS shape; confirm it covers every 772 door.
- **`IsPremiumArea` (`map.cc:2430-2453`) has no equivalent** — undetermined whether the shard needs it.

### Player / combat

This system is in the best shape; what remains is mostly bookkeeping.

- **No death metadata.** `RecordDeath` and `AddKillStatistics` (`crmain.cc:830-860`) plus the `Murderer` field (`crplayer.cc:1546`) have no counterpart — there is no DB death row and no kill statistics. Only the last-hit name reaches the corpse description.
- **Armor slot check is a proxy.** `crcombat.cc:295-297` gates on the CLOTHES and ARMOR flags; `values.rs:288-291` substitutes `armor > 0`.
- **Soul timer does not persist.** `soul` is saved (`game_world_save.rs:139`) but `soul_cycle` / `count` / `max_count` are session-only, so the timer resets on relog.
- **Attack rearm snapback is incomplete.** No player `CreatureMoveStimulus` snapback when the chase target walks away (`crmain.cc:920-965`); tracked as L3/S5 in `docs/SNAPBACK_KNOCKBACK_AUDIT.md`.
- **Death skill-loss abort quirk.** `TSkillLevel::Decrease` aborts when `Amount > Exp && Exp > 100000` (`crskill.cc:300-303`); Rust applies this only on `remove_experience` (`player.rs:476-477`), not in the death skill loop.
- **Latent TFS leak in party XP — fixed with §1.2.** `split_shared_experience` (`party.rs`) even-divides; 772 `DistributeExperiencePoints` has no party bonus (`crcombat.cc:906-921`).
- **`WriteKillStatistics` (`main.cc:394`) is not ported.**

---

## Tier 3 — Script probability and threshold drift

Small, low-risk, and independently verifiable. Each is a data-pack edit.

| Item | Corpus | Current | File |
|---|---|---|---|
| Food cap boundary | `(cur+add) > Max` (`moveuse.cc:1842`) | `>= 1200` — rejects exactly 1200 | `food.lua:53` |
| Birdcage empty chance | nested roll = 0.1% | `random(100)==1` = 1% | `birdcage.lua:4` |
| Waterpipe puff target | 90% item / 10% player | 33% / 67% | `waterpipe.lua:4-7` |
| Didgeridoo success | 10% | 20% | `music.lua:27` |
| Cornucopia grape keep | 95% | 80% | `music.lua:21`, `:29` |
| Change gold | absent on 772 | still registers | `change_gold.lua` |
| Cuckoo clock | use announces time only | also toggles | `decayto.lua:3-4` |
| Teleport PZ cancel | not in corpus | cancels in PZ | `teleport.lua:18-21` |

These are already listed in `tasks/other-actions-plan.md` step 5; this audit confirms them against the corpus.

---

## Intentional deviations

Recorded so future audits do not re-file them as gaps.

- **`moveuse.dat` rule engine is not ported.** `HandleEvent` / `CheckCondition` (26 condition types) / `ExecuteAction` (38 action types) (`moveuse.cc:86-350`, `:946-1531`) are replaced by TFS `Action()` / `MoveEvent()` Lua plus native handlers, per `tasks/movements-plan.md:148`. **The conversion has been done, and systematically** — see [Coverage of the converted dat rules](#coverage-of-the-converted-dat-rules) below. Coordinate-pinned rules became action-id-keyed `MoveEvent` scripts, with the coordinate living in the OTBM as an action id.
- **`playerSpeed = "balanced"`** in `data/formulas/772.lua:49` is a deliberate shard-tuning choice, not a parity bug. The corpus formula is linear `2*Go + 80` (`crskill.cc:667`), available as `playerSpeed = "772"` if strict parity is ever wanted.
- **`script.cc` binary script I/O** is replaced by OTBM + Lua.
- **Rule violation reporting** (corpus channel 3, `operate.cc:3222+`) is an explicit non-goal.
- **Critical hits and stamina do not exist in the 772 corpus.** The stamina DB field is persisted for TFS compatibility but has no gameplay effect.
- **Party channel** exists in Rust (`chat.rs:108`) but not in the corpus public-channel enum (`operate.hh:26-36`) — treat as a gated TFS extra.
- **`change_target` interval/chance, `<elements>` modifiers, `static_attack_chance`, `immunity_outfit`** are TFS XML fields with no corpus equivalent; stored but inert on 772.
- **1098-era opcodes** (market, modal window, wrap, browse field, mount, quest log) are correctly unhandled on 772; they should be version-gated rather than left to the catch-all.

---

## Recommended next steps

Ordered by gameplay impact per unit of effort. Steps 1–5 (trade, party, shop, VIP, rune/spell gates, chat) are done. Step 6 LifeEndRound + monsterhome despawn are done (`DistanceFighting` deferred). Step 7 script numerics is next.

### ~~Step 1 — Player trade~~ **Done (audit 1.1, August 2026)**

Shipped in `trade.rs`: four packet handlers, ToDo `TDTrade`, wire encode, `NotifyTrades`, walk cancel, `house:startTrade` / `!sellhouse` on the same engine. See [§1.1](#11-player-to-player-trade--done).

### ~~Step 2 — Party lifecycle~~ **Done (audit 1.2, August 2026)**

Shipped in `party.rs`: invite / revoke / join / pass leadership / leave / disband, invited-player tracking, shield/skull broadcast, even XP split. See [§1.2](#12-party-lifecycle--done).

### ~~Step 3a — NPC shop~~ **Done (audit 1.3, September 2026)**

Shipped in `shop.rs`: four packet handlers, `shop_owner` / catalog, buy/sell money+capacity, `updateSaleShopList`, Lua `openShopWindow`. See [§1.3](#13-npc-shop-runtime--done).

### ~~Step 3b — VIP runtime~~ **Done (audit 1.4, September 2026)**

Shipped in `vip.rs`: add / remove / edit, `getMaxVIPEntries`, immediate `account_viplist` persist, login/logout status. See [§1.4](#14-vip-runtime--done).

### ~~Step 4 — Rune and spell gates~~ **Done (September 2026)**

Shipped: `CheckRuneLevel` + `EarliestSpellTime` + aggressive PZ on `player_cast_rune`; `CheckAccount` premium on `player_say_spell`; stacked-tile aggressive/self preference via `prefer_rune_tile_target`. See [Magic / spells](#magic--spells).

### ~~Step 5 — Chat parity pass~~ **Done (September 2026)**

Shipped in [`chat_talk.rs`](../crates/tfs-rust-core/src/chat_talk.rs): 7×5 say / 30×30 yell, RecordTalk flood, pack Trade id 6 + 120-round gate, `RecordMessage` PM cap, guild look + guild-channel filter, cancel texts (premium private channel, invite/exclude, EditText NOROOM, UseWithCreature OOR). Flood decision: **port RecordTalk** (not TFS `maxMessageBuffer`). Lua `canJoin`/`onSpeak` still stubbed. See [Player operations and chat](#player-operations-and-chat).

### ~~Step 6 — Monster AI edge paths~~ **Partial (September 2026)**

**Done:** monsterhome idle despawn (`monsterhome_in_range` + `IdleStimulus`) and `LifeEndRound` at the idle entry point (`crnonpl.cc:2352`, `:2407`). Raid tick no longer polls expiry. See [Monster / NPC AI](#monster--npc-ai).

**Remaining (deferred):** explicit `DistanceFighting` flag on `MonsterType` / `RaceData`. Idle still infers the distance branch from `target_distance > 1 && ThrowPossible`.

### Step 7 — Script numerics

The whole Tier 3 table in one pass. Independently testable, no engine risk.

### Step 8 — Splash layer and elevation

Both have standing write-ups (`772_SPLASH_LAYER_MISMATCH.md`, `772_ELEVATION_WALK_PARITY.md` §4/§5). Fixing the splash layer also lets the content-side guards be deleted.

### Step 9 — Death metadata and persistence

`RecordDeath` DB row, kill statistics, armor slot flags, soul timer fields, and the skill-decrease abort quirk.

### Step 10 — Longer tail

Mail (`SendMail`), live sector refresh, house policy evictions and transfer, `MOVEMENTEVENT` hook, `UseAnnouncer` cases 1 and 3, UNLAY shuffle, and NPC `Bless` / `Town` / `String` / `Promote`.

### Step 11 — Spot-check fidelity of the converted dat rules

Not a coverage hunt — the conversion is done (see below). What remains is verifying that each converted rule is faithful, and that the action ids are actually placed on the right OTBM tiles. Start with the `level_2_bridge.lua` offset question recorded below.

---

## Coverage of the converted dat rules

The `moveuse.dat` Collision table was converted into action-id-keyed `MoveEvent` scripts rather than left unimplemented. Evidence:

- **124 distinct action ids, contiguous `3000`–`3123` with no gaps**, across 130 files in `data/scripts/movements/`. A contiguous block is a deliberate allocation pass, not incremental growth.
- **999 hardcoded real-map coordinates** in those scripts, confirming the OTBM is a real-map replica and the corpus coordinates carried across. `tasks/movements-plan.md:142` ("772 Collision-by-coord is not an OT map") is therefore too pessimistic as written.
- Coordinate crosswalk between dat Collision sections and script coordinates lands at 80–100% for most quest and teleporter sections, and exactly 100% for Paradox Tower, Annihilator, Dark Cathedral, Sacrificial Basins and Stones, Desert Quest, Edron Demons, Lighthouse Thais, and Teleporters Absolute/Annihilator.

Worked example — Rookgaard premium bridge. Corpus (`moveuse.dat:1159-1162`):

```
Collision, IsType (Obj1,452), IsPosition (Obj1,[32057,32192,07]), IsPlayer (Obj2), HasRight (Obj2,PREMIUM_ACCOUNT) -> NOP
Collision, IsType (Obj1,452), IsPosition (Obj1,[32057,32192,07]) -> MoveTop(Obj1,[32060,32192,07]), EffectOnMap([32060,32192,07],13)
```

Script (`data/scripts/movements/map/rookgaard/premium_bridge.lua:3-8`) reproduces it exactly — `x + 3`, same `y`, `z = 07`, effect 13 — with the `NOP` fallthrough ladder expressed as an inverted `if not isPremium()`.

### Rule categories by disposition

- **Coordinate-pinned (647 of 828 Collision rules, 561 distinct coordinates)** — converted to aid scripts as above.
- **Type-keyed (181 rules)** — handled natively. Trap Damage (35 rules) via `trap.lua` + `magic_field.rs`; Liquid Deletions (70) and Teleporters Relative (71) via `tile_specials.rs`; Dustbins (1) likewise. These sections contain no coordinates at all, which is why the crosswalk reports none for them.
- **Genuinely uncovered:** `Collision/Mailboxes` (2 rules), which needs `SendMail` — already tracked as Tier 1 item 1.5.

### Residual risk

The open question is no longer whether a handler exists, but whether each conversion is faithful and whether the action id is placed on the correct OTBM tile. A script registered on aid 3051 does nothing if no tile carries 3051.

One concrete discrepancy to check first. Corpus level-2 bridge (`moveuse.dat:1155-1158`) relocates to the **same x**, `y + 1`, `z 07`:

```
Collision, IsType (Obj1,452), IsPosition (Obj1,[32092,32175,06]) -> MoveTop(Obj1,[32092,32176,07])
Collision, IsType (Obj1,452), IsPosition (Obj1,[32091,32175,06]) -> MoveTop(Obj1,[32091,32176,07])
```

`level_2_bridge.lua:5` uses `x = item:getPosition().x - 1`. That is correct only if the aid-3051 tiles sit one tile east of the corpus coordinates; otherwise every drop lands one tile west. Note the sibling `premium_bridge.lua` uses a pure relative offset with no such shift. Resolve by dumping the aid-3051 tile coordinates from the OTBM and comparing against `32091`/`32092`.

A useful general check is the same dump for all 124 aids: any aid with a registered script but no tile in the map is a dead script, and any tile carrying an aid outside `3000`–`3123` is an unhandled trigger.

---

## Verification

```
rtk cargo check --workspace
rtk cargo clippy --workspace --all-targets
rtk cargo test --workspace
rtk cargo run -p tfs-rust-lua --bin emit-lua-defs -- --check
bash scripts/check_data_pack_policy.sh
```

Per-area suites worth running while working the steps above:

```
rtk cargo test -p tfs-rust-core --lib trade
rtk cargo test -p tfs-rust-core --lib party
rtk cargo test -p tfs-rust-core --lib shop
rtk cargo test -p tfs-rust-core --lib vip
rtk cargo test -p tfs-rust-core --lib spell::tests
rtk cargo test -p tfs-rust-core --lib chat_talk
rtk cargo test -p tfs-rust-core --lib idle_stimulus
rtk cargo test -p tfs-rust-core --lib monster_ai
rtk cargo test -p tfs-rust-core --lib player::combat
rtk cargo test -p tfs-rust-net --test protocol_compat
```
